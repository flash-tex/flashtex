//! PDF output through the `pdf` sibling.
//!
//! One route. [`write_pdf_exact`] takes the v2 display list through the pdf
//! crate's `v2` adapter and `exact` writer in-process: glyph runs by original
//! glyph id, font programs embedded as GID-preserving subsets, typed rules,
//! `path_fill`/`path_stroke` and image XObjects. These are the bytes
//! `flashtex-pdf-exact from-v2` produces for the Mac app's Export PDF and what
//! `flashtex build` writes, so `flashtex-render --pdf` now writes them too.
//!
//! There used to be a second route, `write_pdf`, behind that same `--pdf`
//! flag: a self-described SHIM over the runtime-v1 fallback that re-encoded
//! text by character, advanced with its own widths and drew math glyphs from
//! Latin Modern Roman/Symbol rather than Latin Modern Math. It was the last
//! consumer of `flashtex_pdf`'s runtime-v1 surface. Removed — one flag, one
//! set of bytes, and nothing left claiming to export a document it can only
//! approximate.

use std::path::Path;

use crate::display::DisplayList;

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
