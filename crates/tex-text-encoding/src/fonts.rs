//! NFSS font selection for text encodings, and the glyph names behind TFM slots.
//!
//! Sources:
//! * `ot1cmr.fd` (cmr: `<5>…<10><12>gen*cmr`, `<10.95>cmr10`, `<14.4>cmr12`, `<17.28>…cmr17`),
//!   `t1cmr.fd` / `ts1cmr.fd` (`\EC@family` → `ecrm`/`tcrm` + 4-digit size code),
//!   `omscmr.fd` (ssub `cmsy`), `omlcmr.fd` (ssub `cmm` → `cmmi`).
//! * `lmodern.sty` sets `\rmdefault` to `lmr`; `ot1lmr.fd`, `t1lmr.fd`, `ts1lmr.fd`
//!   (`rm-lmr*`, `ec-lmr*`, `ts1-lmr*`: `<-5.5>5 <5.5-6.5>6 … <9.5-11>10 <11-15>12 <15->17`),
//!   `omslmr.fd` (ssub `lmsy`), `omllmr.fd` (ssub `lmm` → `lmmi`).
//! * `pdftex.map`: which Type 1 font and `.enc` vector realise each TFM.

use crate::encoding::Encoding;
use crate::generated::{
    BUILTIN_CMMI10, BUILTIN_CMR10, BUILTIN_CMSY10, ENC_CM_SUPER_T1, ENC_CM_SUPER_TS1, ENC_LM_EC,
    ENC_LM_MATHIT, ENC_LM_MATHSY, ENC_LM_RM, ENC_LM_TS1,
};

/// The roman family in force (`\rmdefault`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Family {
    /// LaTeX default `cmr` (Computer Modern / EC / TC via CM-Super Type 1).
    ComputerModern,
    /// `\usepackage{lmodern}` → `lmr`.
    LatinModern,
}

impl Family {
    pub fn nfss_name(self) -> &'static str {
        match self {
            Family::ComputerModern => "cmr",
            Family::LatinModern => "lmr",
        }
    }
}

fn lm_design_size(size_pt: f64) -> u8 {
    match size_pt {
        s if s < 5.5 => 5,
        s if s < 6.5 => 6,
        s if s < 7.5 => 7,
        s if s < 8.5 => 8,
        s if s < 9.5 => 9,
        s if s < 11.0 => 10,
        s if s < 15.0 => 12,
        _ => 17,
    }
}

/// The TFM NFSS loads for medium-series upright text in `enc` at `size_pt`
/// (only the standard LaTeX sizes are modelled for Computer Modern).
pub fn text_tfm(family: Family, enc: Encoding, size_pt: f64) -> Option<String> {
    match family {
        Family::LatinModern => {
            let n = lm_design_size(size_pt);
            Some(match enc {
                Encoding::OT1 => format!("rm-lmr{n}"),
                Encoding::T1 => format!("ec-lmr{n}"),
                Encoding::TS1 => format!("ts1-lmr{n}"),
                // omslmr.fd / omllmr.fd substitute the math fonts (lmsy 5–10, lmmi 5–12).
                Encoding::OMS => format!("lmsy{}", if n >= 10 { 10 } else { n }),
                Encoding::OML => format!("lmmi{}", n.min(12)),
            })
        }
        Family::ComputerModern => {
            let sizes: [(f64, u16); 12] = [
                (5.0, 500),
                (6.0, 600),
                (7.0, 700),
                (8.0, 800),
                (9.0, 900),
                (10.0, 1000),
                (10.95, 1095),
                (12.0, 1200),
                (14.4, 1440),
                (17.28, 1728),
                (20.74, 2074),
                (24.88, 2488),
            ];
            let &(pt, code) = sizes.iter().find(|(pt, _)| (pt - size_pt).abs() < 1e-3)?;
            Some(match enc {
                Encoding::OT1 => match code {
                    500..=1000 | 1200 => format!("cmr{}", pt as u32),
                    1095 => "cmr10".into(),
                    1440 => "cmr12".into(),
                    _ => "cmr17".into(),
                },
                Encoding::T1 => format!("ecrm{code:04}"),
                Encoding::TS1 => format!("tcrm{code:04}"),
                Encoding::OMS => format!("cmsy{}", (pt as u32).clamp(5, 10)),
                Encoding::OML => format!("cmmi{}", (pt as u32).clamp(5, 12)),
            })
        }
    }
}

/// Which glyph-name vector realises a TFM in pdfTeX (pdftex.map).
pub fn encoding_vector(tfm: &str) -> Option<&'static [&'static str; 256]> {
    let v: &'static [&'static str; 256] = if tfm.starts_with("rm-lm") {
        &ENC_LM_RM
    } else if tfm.starts_with("ec-lm") {
        &ENC_LM_EC
    } else if tfm.starts_with("ts1-lm") {
        &ENC_LM_TS1
    } else if tfm.starts_with("lmsy") {
        &ENC_LM_MATHSY
    } else if tfm.starts_with("lmmi") {
        &ENC_LM_MATHIT
    } else if tfm.starts_with("ec") {
        &ENC_CM_SUPER_T1
    } else if tfm.starts_with("tc") {
        &ENC_CM_SUPER_TS1
    } else if tfm.starts_with("cmr") {
        // Builtin encoding of the AMS Type 1 CM text fonts (identical layout for all sizes).
        &BUILTIN_CMR10
    } else if tfm.starts_with("cmsy") {
        &BUILTIN_CMSY10
    } else if tfm.starts_with("cmmi") {
        &BUILTIN_CMMI10
    } else {
        return None;
    };
    Some(v)
}

/// PostScript glyph name pdfTeX shows for `code` in `tfm`.
pub fn glyph_name(tfm: &str, code: u8) -> Option<&'static str> {
    encoding_vector(tfm)
        .map(|v| v[code as usize])
        .filter(|n| *n != ".notdef")
}

fn digits_of(tfm: &str) -> &str {
    let trimmed = tfm.trim_end_matches(|c: char| c.is_ascii_digit());
    &tfm[trimmed.len()..]
}

/// `/BaseFont` (without subset tag) pdfTeX embeds for a TFM (pdftex.map).
pub fn pdf_base_font(tfm: &str) -> Option<String> {
    let d = digits_of(tfm);
    Some(
        if tfm.starts_with("rm-lmr") || tfm.starts_with("ec-lmr") || tfm.starts_with("ts1-lmr") {
            format!("LMRoman{d}-Regular")
        } else if tfm.starts_with("lmsy") {
            format!("LMMathSymbols{d}-Regular")
        } else if tfm.starts_with("lmmi") {
            format!("LMMathItalic{d}-Regular")
        } else if tfm.starts_with("ecrm") || tfm.starts_with("tcrm") {
            format!("SFRM{d}")
        } else if tfm.starts_with("cm") {
            tfm.to_ascii_uppercase()
        } else {
            return None;
        },
    )
}

/// A bundled OpenType face (apps/mac/Fonts) the IDE can draw a TFM glyph with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundledGlyph {
    pub file: String,
    pub glyph: &'static str,
    /// `false` when the TFM is not Latin Modern (Computer Modern / CM-Super shapes
    /// are approximated by the Latin Modern master of the same design size).
    pub same_design: bool,
}

/// Type 1 (`.enc`) glyph names whose CFF name differs in the LM 2.004 OTFs.
pub fn otf_glyph_alias(name: &'static str) -> &'static str {
    match name {
        "ff" => "f_f",
        "fi" => "f_i",
        "fl" => "f_l",
        "ffi" => "f_f_i",
        "ffl" => "f_f_l",
        "IJ" => "I_J",
        // T1 slot 223 (\SS) is the two-letter "SS" glyph in EC/LM (lm-ec.enc
        // `Germandbls`, cm-super-t1.enc `SS`); the OTF names it `S_S`.
        "Germandbls" | "SS" => "S_S",
        "ij" => "i_j",
        "arrowleft" => "uni2190",
        "arrowup" => "uni2191",
        "arrowright" => "uni2192",
        "arrowdown" => "uni2193",
        "asteriskmath" => "asterisk.math",
        "mho" => "uni2127",
        "musicalnote" => "uni266A",
        "mu" => "uni00B5",
        other => other,
    }
}

const BUNDLED_ROMAN_SIZES: [u8; 8] = [5, 6, 7, 8, 9, 10, 12, 17];

/// Maps a TFM slot to a glyph in the bundled Latin Modern Roman OTFs, if the
/// font is a roman text font (OT1/T1/TS1). Math symbol fonts are not bundled.
pub fn bundled_glyph(tfm: &str, code: u8) -> Option<BundledGlyph> {
    let lm = tfm.starts_with("rm-lmr") || tfm.starts_with("ec-lmr") || tfm.starts_with("ts1-lmr");
    let cm_text = tfm.starts_with("cmr") || tfm.starts_with("ecrm") || tfm.starts_with("tcrm");
    if !lm && !cm_text {
        return None;
    }
    let d: u32 = digits_of(tfm).parse().ok()?;
    let pt = if d >= 100 { d / 100 } else { d } as u8;
    let size = *BUNDLED_ROMAN_SIZES
        .iter()
        .min_by_key(|s| (**s as i32 - pt as i32).abs())?;
    let name = glyph_name(tfm, code)?;
    Some(BundledGlyph {
        file: format!("lmroman{size}-regular.otf"),
        glyph: otf_glyph_alias(name),
        same_design: lm && size == pt,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selects_the_fd_fonts() {
        assert_eq!(
            text_tfm(Family::ComputerModern, Encoding::OT1, 10.0).unwrap(),
            "cmr10"
        );
        assert_eq!(
            text_tfm(Family::ComputerModern, Encoding::T1, 10.0).unwrap(),
            "ecrm1000"
        );
        assert_eq!(
            text_tfm(Family::ComputerModern, Encoding::TS1, 5.0).unwrap(),
            "tcrm0500"
        );
        assert_eq!(
            text_tfm(Family::LatinModern, Encoding::T1, 10.0).unwrap(),
            "ec-lmr10"
        );
        assert_eq!(
            text_tfm(Family::LatinModern, Encoding::OT1, 5.0).unwrap(),
            "rm-lmr5"
        );
        assert_eq!(glyph_name("ec-lmr10", 233), Some("eacute"));
        assert_eq!(glyph_name("cmr10", 25), Some("germandbls"));
        assert_eq!(pdf_base_font("ts1-lmr10").unwrap(), "LMRoman10-Regular");
        assert_eq!(bundled_glyph("rm-lmr10", 12).unwrap().glyph, "f_i");
    }
}
