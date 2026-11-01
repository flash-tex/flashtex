//! The AMS symbol fonts behind `amssymb`/`amsfonts` (`msam`, `msbm`).
//!
//! `amsfonts.sty` declares them as the `AMSa` (`U/msa/m/n`) and `AMSb`
//! (`U/msb/m/n`) symbol fonts; `umsa.fd`/`umsb.fd` load `msam5` below 6 pt,
//! `msam7` from 6 pt up to (not including) 8 pt, and `msam10` from 8 pt
//! (msbm alike), each scaled to the requested size. [`glyph`] returns the
//! character box pdfLaTeX sets for a slot at a math size, from the embedded
//! TFMs of [`crate::ams_tfm`]; which outline draws it is the renderer's
//! business.

use crate::ams_tfm::{MSAM5, MSAM7, MSAM10, MSBM5, MSBM7, MSBM10};
use crate::metrics::{FontId, Glyph};
use crate::tfm::{TfmFont, scale};

/// `AMSa` (`msam`) or `AMSb` (`msbm`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AmsFont {
    Msam,
    Msbm,
}

/// The TFM `umsa.fd`/`umsb.fd` select for a font size in pt.
pub fn tfm(font: AmsFont, size_pt: f64) -> &'static TfmFont {
    match (font, size_pt) {
        (AmsFont::Msam, s) if s < 6.0 => &MSAM5,
        (AmsFont::Msam, s) if s < 8.0 => &MSAM7,
        (AmsFont::Msam, _) => &MSAM10,
        (AmsFont::Msbm, s) if s < 6.0 => &MSBM5,
        (AmsFont::Msbm, s) if s < 8.0 => &MSBM7,
        (AmsFont::Msbm, _) => &MSBM10,
    }
}

/// The box of `slot` in `font` at `size_pt`, tagged `font_id`/`ch` for the
/// renderer (`gid` is the slot). `None` when the slot is empty.
pub fn glyph(font: AmsFont, slot: u8, ch: char, size_pt: f64, font_id: FontId) -> Option<Glyph> {
    let c = tfm(font, size_pt).char(slot)?;
    Some(Glyph {
        font_id,
        gid: u16::from(slot),
        ch,
        size: size_pt,
        width: scale(c.width, size_pt),
        height: scale(c.height, size_pt),
        depth: scale(c.depth, size_pt),
        italic: scale(c.italic, size_pt),
        skew: 0.0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fd_size_ranges_pick_the_design() {
        assert_eq!(tfm(AmsFont::Msam, 5.0).name, "msam5");
        assert_eq!(tfm(AmsFont::Msam, 6.0).name, "msam7");
        assert_eq!(tfm(AmsFont::Msam, 7.0).name, "msam7");
        assert_eq!(tfm(AmsFont::Msbm, 8.0).name, "msbm10");
        assert_eq!(tfm(AmsFont::Msbm, 12.0).name, "msbm10");
    }

    #[test]
    fn leqslant_box_matches_msam10() {
        // msam10.tfm char "36 (\leqslant): CHARWD R 0.777781 at 10pt.
        let g = glyph(AmsFont::Msam, 0x36, '⩽', 10.0, FontId(7)).expect("slot 0x36");
        assert!((g.width - 7.77781).abs() < 1e-4, "{}", g.width);
        assert_eq!(g.gid, 0x36);
    }
}
