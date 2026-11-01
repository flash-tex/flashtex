//! `flashtex_pdf` adapter: hands the PDF writer this crate's embedding
//! output in the shape it already consumes (`embed::EmbeddedSubset`), so the
//! writer stops re-parsing and re-subsetting fonts on its own.
//!
//! Limitation of the target contract: `EmbeddedSubset::chars` is
//! `char -> CID`, one code point per glyph. Ligature and mark clusters
//! (`"fi"`, `"e\u{301}"`) have no single char, so they are not in `chars`;
//! their CIDs are still in the program and `/W`, and this crate's own
//! `to_unicode_cmap` (multi-character `bfchar`) plus per-cluster `text` for
//! `ActualText` are exposed on [`PdfFontProgram`] for the writer to adopt.
//! Until it does, text written through `chars` alone loses ligatures — that
//! is a writer limitation and is stated in docs/consumers.md.

use std::collections::BTreeMap;

use flashtex_pdf::embed::{Descriptor, EmbeddedSubset, Program};
use flashtex_pdf::truetype::Subset as PdfSubset;

use crate::embed::{FontFile, PdfFontProgram};

/// Converts to the PDF writer's embedded-font input.
pub fn to_pdf_embedded_subset(program: &PdfFontProgram) -> EmbeddedSubset {
    let units_per_em = program.subset.as_ref().map_or(1000, |s| s.units_per_em);
    let pdf_program = match (&program.font_file, &program.subset) {
        (FontFile::TrueTypeSubset(bytes), Some(sub)) => Program::TrueType(PdfSubset {
            bytes: bytes.clone(),
            glyph_map: sub.old_to_new.clone(),
            advances: sub.advances.clone(),
        }),
        (FontFile::Cff(bytes), _) => {
            // /W here is 1/1000 em already; the writer rescales by
            // 1000/units_per_em, so hand it font units at 1000/em.
            let used_advances: BTreeMap<u16, u16> = program
                .cid_widths
                .iter()
                .map(|(cid, w)| (*cid, (*w).clamp(0, i32::from(u16::MAX)) as u16))
                .collect();
            Program::Cff {
                bytes: bytes.clone(),
                used_advances,
            }
        }
        (FontFile::TrueTypeSubset(bytes), None) => Program::TrueType(PdfSubset {
            bytes: bytes.clone(),
            glyph_map: program.glyph_map.clone(),
            advances: Vec::new(),
        }),
    };
    let units_per_em = match &pdf_program {
        Program::Cff { .. } => 1000,
        Program::TrueType(_) => units_per_em,
    };
    let mut chars = BTreeMap::new();
    for (cid, text) in &program.to_unicode {
        let mut it = text.chars();
        if let (Some(c), None) = (it.next(), it.next()) {
            chars.entry(c).or_insert(*cid);
        }
    }
    let d = &program.descriptor;
    EmbeddedSubset {
        program: pdf_program,
        units_per_em,
        chars,
        base_font: program.base_font.clone(),
        descriptor: Descriptor {
            bbox: d.bbox,
            ascent: d.ascent,
            descent: d.descent,
            cap_height: d.cap_height,
            italic_angle: d.italic_angle,
        },
    }
}
