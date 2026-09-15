//! PDF output through the `pdf` sibling.
//!
//! Two routes. [`write_pdf_exact`] is the exact route (vendor/pdf re-pinned
//! to main's `crates/pdf`): the v2 display list goes through the pdf
//! crate's `v2` adapter and `exact` writer in-process — the bytes
//! `flashtex-pdf-exact from-v2` produces for the app's Export PDF, and what
//! `flashtex build` writes. [`write_pdf`] is the older `flashtex-render
//! --pdf` shim, kept for that flag:
//!
//! that route consumes runtime-v1 pages on the negotiated capabilities
//! (`rules-v1`, `font-hints-v1`): typed rules are drawn as filled
//! rectangles from their real geometry, and font hints pick the Latin
//! Modern regular/bold/italic faces (each embedded whole from the local
//! Latin Modern directory). It has no glyph-run API yet, so this is still a
//! SHIM over the v1 fallback: positions are the pipeline's, but the writer
//! re-encodes text by character and advances with its own widths, and math
//! glyphs are drawn from Latin Modern Roman/Symbol rather than Latin Modern
//! Math. Requested pdf API: accept v2 glyph runs (font id + original GIDs +
//! positions) — see docs/proposals/rendering-abi.md.

use std::path::Path;

use flashtex_pdf::embed::EmbedFont;
use flashtex_pdf::{CompileResult, FontHint, Item, Page, RenderOptions as PdfOptions, RuleItem, Style, TextItem, Weight};

use crate::display::DisplayList;
use crate::v1::{self, Capabilities, V1Item, CAP_FONT_HINTS, CAP_RULES};

pub struct PdfOut {
    pub bytes: Vec<u8>,
    pub warnings: Vec<String>,
    /// PostScript name of the embedded document face, if any.
    pub embedded: Option<String>,
}

/// What the exact route embedded, for a CLI summary line.
pub struct ExactPdfOut {
    pub bytes: Vec<u8>,
    /// One line per embedded font (`/F1 LMRoman12-Regular from …`) and the
    /// route's own notes, in the order `flashtex-pdf-exact from-v2` prints.
    pub notes: Vec<String>,
    pub fonts: usize,
    pub glyphs: usize,
    pub images: usize,
}

/// The exact route in-process: the display list serialised as the
/// `display_list` envelope (`display-list-v2`, images included) and read
/// back by the pdf sibling's `v2` adapter — glyph runs by original glyph id
/// at exact tick positions, fonts resolved by content hash and embedded as
/// GID-preserving subsets, typed rules, images and links — then rendered by
/// `exact::render_exact` and structurally self-checked. This is the PDF the
/// app's "Export PDF" produces through `flashtex-pdf-exact from-v2`.
///
/// `font_dirs` are probed first for the font bytes; `project_root` is the
/// directory `image` items resolve under (`None` refuses images).
pub fn write_pdf_exact(v2: &DisplayList, font_dirs: &[std::path::PathBuf], project_root: Option<&Path>) -> Result<ExactPdfOut, String> {
    // A PDF is a complete document: it never silently omits a page. A
    // windowed render is an incomplete view, so it is refused rather than
    // exported short (`protocol/proposals/display-list-v2-window.md` §5.6).
    if let Some(w) = v2.window {
        return Err(format!(
            "cannot export a windowed render: pages {}-{} of {} were materialised; re-render without a page window",
            w.first_page,
            w.first_page + w.page_count - 1,
            v2.pages.len()
        ));
    }
    let envelope = v2.write_json_with("export", true);
    let options = flashtex_pdf::v2::V2Options { font_dirs: font_dirs.to_vec() };
    let (doc, report) = flashtex_pdf::v2::from_v2_rooted(&envelope, &options, project_root)?;
    let rendered = flashtex_pdf::exact::render_exact(&doc).map_err(|e| e.to_string())?;
    flashtex_pdf::verify::check_structure(&rendered.bytes).map_err(|e| format!("generated PDF failed self-check: {e}"))?;
    let mut notes = Vec::new();
    for f in &report.fonts {
        notes.push(format!(
            "/{} {} from {}: {} glyph(s), {:?}, program {} bytes{}",
            f.resource,
            f.postscript_name,
            f.path.display(),
            f.glyphs,
            f.outcome,
            f.program_bytes,
            f.note.as_deref().map_or(String::new(), |n| format!(" ({n})"))
        ));
    }
    notes.extend(report.notes.iter().cloned());
    Ok(ExactPdfOut {
        bytes: rendered.bytes,
        notes,
        fonts: report.fonts.len(),
        glyphs: report.glyphs,
        images: report.images,
    })
}

fn hint(h: &v1::FontHint) -> FontHint {
    FontHint {
        family: h.family.to_string(),
        weight: if h.weight == "bold" { Weight::Bold } else { Weight::Normal },
        style: if h.style == "italic" { Style::Italic } else { Style::Normal },
    }
}

pub fn write_pdf(v2: &DisplayList) -> Result<PdfOut, String> {
    // A PDF is a complete document: it never silently omits a page. A
    // windowed render is an incomplete view, so it is refused rather than
    // exported short (`protocol/proposals/display-list-v2-window.md` §5.6).
    if let Some(w) = v2.window {
        return Err(format!(
            "cannot export a windowed render: pages {}-{} of {} were materialised; re-render without a page window",
            w.first_page,
            w.first_page + w.page_count - 1,
            v2.pages.len()
        ));
    }
    let caps = Capabilities {
        images: false,
        rules: true,
        font_hints: true,
        display_list: false,
        device_color: false,
        ..Capabilities::default()
    };
    let accepted = vec![CAP_RULES.to_string(), CAP_FONT_HINTS.to_string()];
    let v1 = v1::fallback(v2, caps, Some(accepted.clone()));
    let pages = v1
        .pages
        .iter()
        .map(|p| Page {
            number: p.number,
            width_pt: p.width_pt,
            height_pt: p.height_pt,
            items: p
                .items
                .iter()
                .map(|it| match it {
                    V1Item::Text {
                        text,
                        x_pt,
                        baseline_y_pt,
                        font_size_pt,
                        font,
                        ..
                    } => Item::Text(TextItem {
                        text: text.clone(),
                        x_pt: *x_pt,
                        baseline_y_pt: *baseline_y_pt,
                        font_size_pt: *font_size_pt,
                        font: font.as_ref().map(hint),
                    }),
                    V1Item::Rule {
                        x_pt,
                        y_pt,
                        width_pt,
                        height_pt,
                        ..
                    } => Item::Rule(RuleItem {
                        x_pt: *x_pt,
                        y_pt: *y_pt,
                        width_pt: *width_pt,
                        height_pt: *height_pt,
                    }),
                })
                .collect(),
        })
        .collect();
    let result = CompileResult {
        pages,
        capabilities: Some(accepted),
    };
    // Embed the regular Latin Modern text face the document used as the
    // document face; the hints select bold/italic siblings from its directory.
    let face = v2
        .fonts
        .iter()
        .filter(|f| f.format == "opentype-cff" && f.postscript_name.starts_with("LMRoman"))
        .find(|f| f.postscript_name.ends_with("-Regular"))
        .or_else(|| v2.fonts.iter().find(|f| f.format == "opentype-cff"));
    let mut warnings = Vec::new();
    let mut embedded = None;
    let options = match face.and_then(|f| f.path.as_deref()) {
        Some(path) => match EmbedFont::load(Path::new(path)) {
            Ok(font) => {
                embedded = Some(font.font.postscript_name.clone());
                PdfOptions::with_document_face(font)
            }
            Err(e) => {
                warnings.push(format!("could not embed {path}: {e}; falling back to Times"));
                PdfOptions::default()
            }
        },
        None => {
            warnings.push("no OpenType face used by the document; Times (unembedded) output".into());
            PdfOptions::default()
        }
    };
    let out = flashtex_pdf::render_pdf_with(&result, &options).map_err(|e| e.to_string())?;
    warnings.extend(out.warnings);
    Ok(PdfOut {
        bytes: out.bytes,
        warnings,
        embedded,
    })
}
