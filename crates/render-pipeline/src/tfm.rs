//! TeX font metrics (`.tfm`) for the text faces, so widths, kerns,
//! ligatures, heights/depths and the interword `\fontdimen`s are the exact
//! fixed-point values pdfTeX lays out with, while the OpenType program of
//! the same face supplies the outlines. A TFM is metrics data from the TeX
//! distribution, like the `.otf` next to it; no TeX engine runs.
//!
//! Parsing and the ligature/kern program are the shared `font-resources`
//! reader (`flashtex_font_resources::tfm::Tfm`, `tfm_run::GlyphRun`, pinned
//! at `vendor/font-resources/PIN`): bounded tables, boundary-character
//! programs on both sides, checked kern sums, a 4096-code run limit. This
//! module keeps the pipeline's view: fixwords (2^-20 of the design size)
//! as the advance unit with `2^20` units per em, so `fixword * size / 2^20`
//! reproduces TeX's `xn_over_d` to within one scaled point; a run's
//! `leading_kern` (a left-boundary kern) is carried as an explicit
//! zero-glyph advance before the first character, never dropped. Errors
//! from the shared interpreter propagate as [`TfmError`]; callers report
//! them and fall back to the font program's own metrics for that run.

use std::path::Path;

use flashtex_font_resources::tfm::{BoundaryOptions, Tfm as SharedTfm};

/// 2^20: fixword units per design em.
pub const FIX: i64 = 1 << 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CharMetrics {
    /// Fixwords.
    pub width: i32,
    pub height: i32,
    pub depth: i32,
    pub italic: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TfmError(pub String);

impl std::fmt::Display for TfmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// The first `lf` words of a TFM file, `lf` being its header's file length
/// in words. TeX (`tex.web` §575) reads exactly `lf` words and never looks
/// past them, and `tftopl` accepts trailing bytes ("extra junk at the end
/// of the TFM file ... proceed as if it weren't there"). The `jknappen/ec`
/// metrics that `t1cmr.fd` loads (`ecrm1095.tfm` and every other EC size)
/// are zero-padded to 3584 bytes, which the shared reader's exact
/// `len == lf * 4` check rejects. A file shorter than `lf` words is passed
/// through unchanged so the reader still reports it as truncated.
fn tex_file_words(b: &[u8]) -> &[u8] {
    if b.len() < 2 {
        return b;
    }
    let lf_bytes = usize::from(u16::from_be_bytes([b[0], b[1]])) * 4;
    if lf_bytes > 0 && b.len() > lf_bytes {
        &b[..lf_bytes]
    } else {
        b
    }
}

#[derive(Debug, Clone)]
pub struct Tfm {
    inner: SharedTfm,
    pub design_size_pt: f64,
}

impl Tfm {
    pub fn load(path: &Path) -> Result<Tfm, String> {
        let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
        Tfm::parse(&bytes).map_err(|e| format!("{}: {e}", path.display()))
    }

    pub fn parse(b: &[u8]) -> Result<Tfm, String> {
        let b = tex_file_words(b);
        let inner = SharedTfm::parse(b).map_err(|e| format!("{e:?}"))?;
        Ok(Tfm::from_shared(inner))
    }

    /// Wraps a TFM the shared loader already verified (a digest-bound
    /// required asset).
    pub fn from_shared(inner: SharedTfm) -> Tfm {
        let design_size_pt = f64::from(inner.design_size.0) / FIX as f64;
        Tfm { inner, design_size_pt }
    }

    /// SHA-256 of the TFM bytes (font-resources' `source_sha256`), for
    /// provenance evidence.
    pub fn sha256(&self) -> &str {
        &self.inner.source_sha256
    }

    /// Whether the font declares a boundary character or a left-boundary
    /// program (both are executed by the shared interpreter).
    pub fn has_boundary(&self) -> bool {
        self.inner.boundary_character.is_some() || self.inner.left_boundary_program.is_some()
    }

    /// Metrics of a character code, `None` when the font has no such
    /// character (never a zero-width guess).
    pub fn metrics(&self, code: u8) -> Option<CharMetrics> {
        let m = self.inner.char_metrics(code)?;
        Some(CharMetrics {
            width: m.width.0,
            height: m.height.0,
            depth: m.depth.0,
            italic: m.italic.0,
        })
    }

    /// The first instruction of `left`'s lig/kern program for `right`
    /// (tex.web §545) as math's `make_ord` reads it (§752): no boundary
    /// characters, no run. `None` without an instruction, or when either
    /// character is missing or the program is malformed.
    #[cfg(feature = "math-font-kerns")]
    pub fn pair_program(&self, left: u8, right: u8) -> Option<flashtex_math_layout::tfm::LigKern> {
        use flashtex_font_resources::tfm::PairAction;
        Some(match self.inner.pair_action(left, right).ok()?? {
            PairAction::Kern(k) => flashtex_math_layout::tfm::LigKern::Kern(k.0),
            PairAction::Ligature { replacement, retain_left, retain_right, advance } => {
                flashtex_math_layout::tfm::LigKern::Ligature { op: advance * 4 + (u8::from(retain_left) << 1) + u8::from(retain_right), rem: replacement }
            }
        })
    }

    /// `\fontdimen n` (1-based) in fixwords.
    pub fn param(&self, n: usize) -> Option<i32> {
        self.inner.parameter(n).map(|v| v.0)
    }

    /// Fixword to points at `size_pt`.
    pub fn pt(fixword: i32, size_pt: f64) -> f64 {
        f64::from(fixword) * size_pt / FIX as f64
    }

    /// Applies the ligature/kern program (both boundaries on, as TeX does
    /// for a word between non-character nodes) to a sequence of codes.
    /// Each output glyph keeps the range of input positions it came from
    /// and the kern that follows it; a left-boundary kern is returned
    /// separately and must be advanced before the first glyph.
    pub fn ligkern(&self, codes: &[u8]) -> Result<TfmRun, TfmError> {
        let run = self
            .inner
            .glyph_run(codes, BoundaryOptions::default())
            .map_err(|e| TfmError(format!("{e:?}")))?;
        Ok(TfmRun {
            leading_kern: run.leading_kern.0,
            glyphs: run
                .glyphs
                .into_iter()
                .map(|g| TfmGlyph {
                    code: g.code,
                    input: (g.input_start, g.input_end),
                    kern_after: g.kern_after.0,
                })
                .collect(),
        })
    }
}

/// The codes with **no** ligature or kern program applied: every character
/// stands for itself, nothing is inserted between two of them, and neither
/// word boundary runs its program.
///
/// This is the regime `\verb`, `verbatim`/`verbatim*` and `lstlisting`
/// typeset under. LaTeX gets there by making `` ` ``, `'`, `<`, `>`, `,`
/// and `-` active inside verbatim (`\@noligs`, latex.ltx), each expanding
/// to `\kern\z@` followed by the character, so the ligature program can
/// never see two of them in a row — visible in pdflatex's `\showbox` as
/// the `\kern 0.0` between the `b` and the `-` of `\verb*"a b-c"`.
///
/// The measured consequence at 12 pt T1 (`ectt1200`, every character
/// 6.1735 pt): `\verb|x--y|` is 24.69397 pt = 4 characters, while
/// `\ttfamily x--y` is 18.52048 pt = 3, because `ectt` *does* carry the
/// `--` → endash ligature and `\ttfamily` is right to use it.
pub fn literal_run(codes: &[u8]) -> TfmRun {
    TfmRun {
        leading_kern: 0,
        glyphs: codes
            .iter()
            .enumerate()
            .map(|(i, &code)| TfmGlyph { code, input: (i, i + 1), kern_after: 0 })
            .collect(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TfmRun {
    /// Fixword kern before the first glyph (left boundary program).
    pub leading_kern: i32,
    pub glyphs: Vec<TfmGlyph>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TfmGlyph {
    pub code: u8,
    /// Half-open range of input positions.
    pub input: (usize, usize),
    /// Fixword kern following this glyph.
    pub kern_after: i32,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lmr12() -> Option<Tfm> {
        let p = crate::fonts::default_tfm_dirs()
            .into_iter()
            .map(|d| d.join("ec-lmr12.tfm"))
            .find(|p| p.is_file())?;
        Tfm::load(&p).ok()
    }

    #[test]
    fn ec_lmr12_widths_ligatures_and_kerns_are_tfms() {
        let Some(t) = lmr12() else {
            eprintln!("skipping: ec-lmr12.tfm not installed");
            return;
        };
        assert_eq!(t.design_size_pt, 12.0);
        assert_eq!(t.sha256().len(), 64);
        // TFtoPL: (CHARACTER C w (CHARWD R 0.707164)) in ec-lmr12.
        let w = t.metrics(b'w').unwrap();
        assert!((Tfm::pt(w.width, 12.0) - 8.486).abs() < 0.001, "{}", Tfm::pt(w.width, 12.0));
        // f f i -> ffi (T1 slot 0x1E) through two ligature steps.
        let g = t.ligkern(b"office").unwrap();
        let codes: Vec<u8> = g.glyphs.iter().map(|g| g.code).collect();
        assert_eq!(codes, vec![b'o', 0x1E, b'c', b'e']);
        assert_eq!(g.glyphs[1].input, (1, 4));
        assert_eq!(g.leading_kern, 0, "ec-lmr12 has no boundary program");
        // "wo" kerns by -0.0272em (TFtoPL: KRN C o R -0.027199).
        let g = t.ligkern(b"wo").unwrap();
        assert!((Tfm::pt(g.glyphs[0].kern_after, 12.0) + 0.326).abs() < 0.002, "{}", Tfm::pt(g.glyphs[0].kern_after, 12.0));
        // Interword glue: \fontdimen2..4 and 7.
        assert!((Tfm::pt(t.param(2).unwrap(), 12.0) - 3.916).abs() < 0.002);
        assert!(!t.has_boundary());
    }

    #[test]
    fn bytes_after_the_declared_file_length_are_ignored_like_tex() {
        let Some(p) = crate::fonts::default_tfm_dirs().into_iter().map(|d| d.join("ec-lmr12.tfm")).find(|p| p.is_file()) else {
            eprintln!("skipping: ec-lmr12.tfm not installed");
            return;
        };
        let exact = std::fs::read(&p).unwrap();
        let mut padded = exact.clone();
        padded.extend_from_slice(&[0u8; 412]);
        let a = Tfm::parse(&exact).unwrap();
        let b = Tfm::parse(&padded).expect("trailing bytes are not part of the TFM");
        assert_eq!(a.metrics(b'w'), b.metrics(b'w'));
        assert_eq!(a.param(2), b.param(2));
        // A file shorter than its declared length is still rejected.
        assert!(Tfm::parse(&exact[..exact.len() - 4]).is_err());
        assert!(Tfm::parse(&[]).is_err());
    }

    #[test]
    fn ecrm1095_the_padded_t1_cmr_body_metrics_loads() {
        let Some(p) = crate::fonts::default_tfm_dirs().into_iter().map(|d| d.join("ecrm1095.tfm")).find(|p| p.is_file()) else {
            eprintln!("skipping: ecrm1095.tfm not installed");
            return;
        };
        // jknappen/ec files are zero-padded past `lf` words (3584 bytes).
        let bytes = std::fs::read(&p).unwrap();
        assert!(bytes.len() > usize::from(u16::from_be_bytes([bytes[0], bytes[1]])) * 4);
        let t = Tfm::load(&p).unwrap();
        assert!((t.design_size_pt - 10.95).abs() < 1e-3, "{}", t.design_size_pt);
        // TFtoPL: (SPACE R 0.331557), (CHARACTER C w (CHARWD R 0.713515)).
        assert!((Tfm::pt(t.param(2).unwrap(), 10.95) - 0.331557 * 10.95).abs() < 0.001);
        let w = t.metrics(b'w').unwrap();
        let lm = crate::fonts::default_tfm_dirs().into_iter().map(|d| d.join("ec-lmr10.tfm")).find(|p| p.is_file()).and_then(|p| Tfm::load(&p).ok());
        if let Some(lm) = lm {
            // The EC design is narrower than Latin Modern scaled to 10.95pt.
            assert!(w.width < lm.metrics(b'w').unwrap().width);
        }
    }
}
