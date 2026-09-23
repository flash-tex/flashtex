//! A minimal-but-real TikZ subset compiled into vector display items.
//!
//! This is an original reader for the drawing language of TikZ; no TeX or
//! PGF code runs. It supports `tikzpicture` bodies with `\draw`, `\fill`,
//! `\filldraw`, `\path`, `\clip`, `\node`, `\coordinate`, `\foreach`,
//! `scope`s, `\tikzset`/`\tikzstyle` styles, and the path operations
//! `--`, `-|`, `|-`, `.. controls ..`, `to`, `rectangle`, `circle`,
//! `ellipse`, `arc`, `grid`, `cycle` and path nodes. See `docs/tikz.md` for
//! the exact list and for what is reported as unsupported.
//!
//! Geometry is computed as PGF does it, in TeX points on a y-up canvas, and
//! converted once at the end into the crate's convention: PDF points,
//! top-left origin at the picture's bounding box, y down. Node text is not
//! typeset here: a [`TextMeasurer`] supplied by the caller (the render
//! pipeline's shaper) gives its width, height and depth, and the picture
//! returns [`PictureText`] placements for the caller to paint as glyphs.

pub mod expr;
mod interp;
pub mod text;
pub mod xcolor;

use crate::color::Paint;
use crate::geom::Transform;
use crate::item::Item;

/// Font request for node text.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextStyle {
    pub size_pt: f64,
    pub bold: bool,
    pub italic: bool,
}

/// Box of a typeset string, TeX points.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct TextMetrics {
    pub width_pt: f64,
    pub height_pt: f64,
    pub depth_pt: f64,
}

/// Measures node text. Implemented by the render pipeline with the real
/// fonts; [`ApproxMeasurer`] is a font-free stand-in for tests.
pub trait TextMeasurer {
    fn measure(&self, text: &str, style: &TextStyle) -> TextMetrics;
}

/// Font-free measurer: half an em per character, 0.683 em high, 0.194 em
/// deep when the text has a descender.
pub struct ApproxMeasurer;

impl TextMeasurer for ApproxMeasurer {
    fn measure(&self, text: &str, style: &TextStyle) -> TextMetrics {
        let n = text.chars().count() as f64;
        let deep = text.chars().any(|c| "gjpqy,;()".contains(c));
        TextMetrics {
            width_pt: 0.5 * style.size_pt * n,
            height_pt: if n > 0.0 { 0.683 * style.size_pt } else { 0.0 },
            depth_pt: if deep { 0.194 * style.size_pt } else { 0.0 },
        }
    }
}

/// A piece of node text to paint.
#[derive(Clone, Debug, PartialEq)]
pub struct PictureText {
    pub text: String,
    pub style: TextStyle,
    /// Maps text space (PDF points, origin at the start of the baseline,
    /// y down) into picture space.
    pub transform: Transform,
    pub paint: Paint,
    /// Paint order: the text is painted after `items[..after_item]`.
    pub after_item: usize,
    /// Byte range of the statement that produced it, in the source passed to
    /// [`Tikz::render`].
    pub source: (usize, usize),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Severity {
    Warning,
    Error,
}

/// Something the reader did not implement or could not understand. Nothing
/// unsupported is silently dropped: each skip has one of these.
#[derive(Clone, Debug, PartialEq)]
pub struct Diagnostic {
    pub severity: Severity,
    pub message: String,
    /// Byte range in the source passed to [`Tikz::render`].
    pub start: usize,
    pub end: usize,
}

/// A compiled picture.
#[derive(Clone, Debug, PartialEq)]
pub struct Picture {
    /// Bounding box size in PDF points (PGF's picture size).
    pub width_bp: f64,
    pub height_bp: f64,
    /// Vector items in picture space (PDF points, top-left, y down).
    pub items: Vec<Item>,
    pub texts: Vec<PictureText>,
    pub diagnostics: Vec<Diagnostic>,
}

/// Reader state that outlives one picture: styles and colours defined in
/// the preamble with `\tikzset`, `\tikzstyle`, `\definecolor`, `\colorlet`.
#[derive(Clone, Debug)]
pub struct Tikz {
    /// Document body font size (10 for `standalone`/`article`).
    pub font_size_pt: f64,
    pub(crate) styles: std::collections::HashMap<String, (String, Option<String>)>,
    pub(crate) palette: xcolor::Palette,
}

impl Default for Tikz {
    fn default() -> Self {
        Tikz::new(10.0)
    }
}

/// One `tikzpicture` environment found in a document.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PictureSource {
    /// Byte range of `\begin{tikzpicture}` .. `\end{tikzpicture}`.
    pub start: usize,
    pub end: usize,
    /// Byte range of the body (after the optional `[options]`).
    pub body_start: usize,
    pub body_end: usize,
    /// Byte range of the options text (inside the brackets), if any.
    pub options: Option<(usize, usize)>,
}

/// Finds the `tikzpicture` environments (and `\tikz{...}`/`\tikz ...;` is
/// not recognised) in a document, skipping `%` comments.
pub fn find_pictures(doc: &str) -> Vec<PictureSource> {
    let clean = text::blank_comments(doc);
    let clean = clean.as_ref();
    let begin = "\\begin{tikzpicture}";
    let end = "\\end{tikzpicture}";
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(rel) = clean[from..].find(begin) {
        let start = from + rel;
        let mut body_start = start + begin.len();
        let mut options = None;
        let after = clean[body_start..].trim_start();
        let skipped = clean[body_start..].len() - after.len();
        if after.starts_with('[') {
            let open = body_start + skipped;
            if let Some(close) = text::matching(&clean, open) {
                options = Some((open + 1, close - 1));
                body_start = close;
            }
        }
        let Some(erel) = clean[body_start..].find(end) else {
            break;
        };
        let body_end = body_start + erel;
        out.push(PictureSource {
            start,
            end: body_end + end.len(),
            body_start,
            body_end,
            options,
        });
        from = body_end + end.len();
    }
    out
}

impl Tikz {
    pub fn new(font_size_pt: f64) -> Tikz {
        Tikz {
            font_size_pt,
            styles: Default::default(),
            palette: Default::default(),
        }
    }

    /// Reads `\tikzset`, `\tikzstyle`, `\definecolor` and `\colorlet` from a
    /// preamble (everything else in it is ignored).
    pub fn read_preamble(&mut self, preamble: &str) -> Vec<Diagnostic> {
        let measurer = ApproxMeasurer;
        let mut it = interp::Interp::new(self, &measurer, 0);
        it.preamble(preamble);
        let diags = std::mem::take(&mut it.diags);
        let (styles, palette) = it.into_definitions();
        self.styles = styles;
        self.palette = palette;
        diags
    }

    /// Compiles one picture. `source` is the whole document (spans in the
    /// result index into it); `picture` locates the environment.
    pub fn render(&self, source: &str, picture: &PictureSource, measurer: &dyn TextMeasurer) -> Picture {
        let source_start = picture.start;
        let clean = text::blank_comments(&source[source_start..picture.end]);
        let clean = clean.as_ref();
        let mut it = interp::Interp::new(self, measurer, picture.start);
        let options = picture
            .options
            .map(|(a, b)| &clean[a - source_start..b - source_start])
            .unwrap_or("");
        it.picture(
            options,
            &clean[picture.body_start - source_start..picture.body_end - source_start],
            picture.body_start,
        );
        it.finish()
    }

    /// Convenience: compiles the body of a picture given as a string (no
    /// `\begin{tikzpicture}`), with optional picture options.
    pub fn render_body(&self, options: &str, body: &str, measurer: &dyn TextMeasurer) -> Picture {
        let clean = text::blank_comments(body);
        let mut it = interp::Interp::new(self, measurer, 0);
        it.picture(options, clean.as_ref(), 0);
        it.finish()
    }
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod perf_test;
