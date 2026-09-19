//! Font-dependent TeX parameters (`\fontdimen`s) for the supported faces.
//!
//! These are the interword glue, x-height and quad of the text fonts and the
//! Appendix G math parameters (σ from the symbol font, ξ from the extension
//! font). They were transcribed once from the TFM files that pdfLaTeX uses
//! for the same faces (Latin Modern `ec-lmr*`, `lmsy*`, `lmex10`; Adobe Times
//! `ptm*8t`) with a throwaway TFM reader and are expressed as fractions of the
//! design size so they scale exactly. Reading them at development time is
//! oracle research; nothing reads TFM files at run time.

use crate::fonts::Family;

/// Text-font parameters as em fractions of the *design* size.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextParams {
    pub space: f64,
    pub stretch: f64,
    pub shrink: f64,
    pub x_height: f64,
    pub quad: f64,
    pub extra_space: f64,
}

impl TextParams {
    /// Parameters for a face at `size_pt`, absolute points.
    pub fn at(&self, size_pt: f64) -> TextParamsPt {
        TextParamsPt {
            space: self.space * size_pt,
            stretch: self.stretch * size_pt,
            shrink: self.shrink * size_pt,
            x_height: self.x_height * size_pt,
            quad: self.quad * size_pt,
            extra_space: self.extra_space * size_pt,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextParamsPt {
    pub space: f64,
    pub stretch: f64,
    pub shrink: f64,
    pub x_height: f64,
    pub quad: f64,
    pub extra_space: f64,
}

/// Latin Modern text parameters by design size (`ec-lmr<d>` / `ec-lmbx<d>` /
/// `ec-lmri<d>`). Regular and italic differ; bold is `ec-lmbx12` for the
/// heading sizes used here.
pub fn text_params(family: Family, bold: bool, italic: bool, design_size: u32) -> TextParams {
    match family {
        Family::Times => TextParams {
            // ptmr8t / ptmb8t / ptmri8t (fontinst): identical glue.
            space: 0.25,
            stretch: 0.15,
            shrink: 0.06,
            x_height: if bold { 0.461 } else if italic { 0.441 } else { 0.45 },
            quad: 1.0,
            extra_space: 0.06,
        },
        // Only the fallback when no TFM is attached; Computer Modern (EC)
        // faces normally carry their `ec*` TFM's own `\fontdimen`s. A named
        // family never reaches here: `typeset::Context::text_params` reads
        // its parameters off the OpenType face (`fonts::opentype_params`).
        Family::LatinModern | Family::ComputerModern | Family::Named(_) => {
            if italic {
                // ec-lmri12 (also used for 10: lmri10 has the same fractions
                // to four places).
                TextParams {
                    space: 0.35,
                    stretch: 0.15,
                    shrink: 0.1,
                    x_height: 0.430556,
                    quad: 1.0,
                    extra_space: 0.1,
                }
            } else if bold {
                // ec-lmbx12 (design 12): also used for the scaled bold sizes.
                TextParams {
                    space: 0.375,
                    stretch: 0.1875,
                    shrink: 0.125,
                    x_height: 0.444444,
                    quad: 1.125,
                    extra_space: 0.125,
                }
            } else {
                match design_size {
                    17 => TextParams {
                        space: 0.301889,
                        stretch: 0.156731,
                        shrink: 0.104487,
                        x_height: 0.430497,
                        quad: 0.917235,
                        extra_space: 0.104487,
                    },
                    12 => TextParams {
                        space: 0.326385,
                        stretch: 0.163192,
                        shrink: 0.108795,
                        x_height: 0.430556,
                        quad: 0.979154,
                        extra_space: 0.108795,
                    },
                    8 => TextParams {
                        space: 0.354172,
                        stretch: 0.177086,
                        shrink: 0.118057,
                        x_height: 0.430563,
                        quad: 1.0625,
                        extra_space: 0.118057,
                    },
                    _ => TextParams {
                        // ec-lmr10
                        space: 0.333333,
                        stretch: 0.166667,
                        shrink: 0.111112,
                        x_height: 0.43055,
                        quad: 1.0,
                        extra_space: 0.111112,
                    },
                }
            }
        }
    }
}

/// Appendix G σ parameters of the symbol font, as fractions of its design
/// size, plus the ξ parameters of the extension font.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MathParams {
    pub x_height: f64,
    pub quad: f64,
    pub num1: f64,
    pub num2: f64,
    pub num3: f64,
    pub denom1: f64,
    pub denom2: f64,
    pub sup1: f64,
    pub sup2: f64,
    pub sup3: f64,
    pub sub1: f64,
    pub sub2: f64,
    pub sup_drop: f64,
    pub sub_drop: f64,
    pub delim1: f64,
    pub delim2: f64,
    pub axis_height: f64,
    /// ξ8, as a fraction of the extension font size (`lmex10` at the text size).
    pub default_rule_thickness: f64,
    pub big_op_spacing: [f64; 5],
}

/// Symbol-font parameters for the LaTeX size classes: text (`lmsy10`),
/// script (`lmsy8`) and scriptscript (`lmsy6`). Times documents use the same
/// values through `mathptmx`'s Computer Modern symbol fallback, which is what
/// the `times` package also does.
pub fn math_params(level: u8) -> MathParams {
    let (x_height, quad, num1, num2, num3, denom1, denom2, sup1, sup2, sup3, sub1, sub2, sup_drop, sub_drop, delim1, delim2) =
        match level {
            0 => (
                0.430555, 1.000003, 0.676508, 0.393732, 0.443731, 0.685951, 0.344841, 0.412892,
                0.362892, 0.288889, 0.15, 0.247217, 0.386108, 0.05, 2.389999, 1.01,
            ),
            1 => (
                0.430555, 1.062515, 0.696331, 0.408833, 0.457443, 0.749109, 0.392166, 0.415417,
                0.352917, 0.284721, 0.125, 0.25, 0.395834, 0.0625, 1.487499, 1.137501,
            ),
            _ => (
                0.430554, 1.277771, 0.812698, 0.392331, 0.484924, 0.863622, 0.429364, 0.502968,
                0.419635, 0.287038, 0.166667, 0.333333, 0.412033, 0.083333, 1.983333, 1.350001,
            ),
        };
    MathParams {
        x_height,
        quad,
        num1,
        num2,
        num3,
        denom1,
        denom2,
        sup1,
        sup2,
        sup3,
        sub1,
        sub2,
        sup_drop,
        sub_drop,
        delim1,
        delim2,
        axis_height: 0.25,
        default_rule_thickness: 0.039999,
        big_op_spacing: [0.111112, 0.166667, 0.2, 0.6, 0.1],
    }
}
