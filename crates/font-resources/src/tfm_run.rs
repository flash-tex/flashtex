//! Adapter for consumers storing a kern after each encoded glyph.
//! Leading boundary kerns remain explicit; no boundary effect is discarded.
use crate::{invalid, tfm::*, Result};
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernedGlyph {
    pub code: u8,
    pub input_start: usize,
    pub input_end: usize,
    pub kern_after: FixWord,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlyphRun {
    pub tfm_sha256: String,
    pub leading_kern: FixWord,
    pub glyphs: Vec<KernedGlyph>,
}
impl Tfm {
    /// Same-font encoded run, retaining exact source hash, byte intervals and
    /// signed fix_words. The existing interpreter owns all execution bounds.
    pub fn glyph_run(&self, input: &[u8], boundaries: BoundaryOptions) -> Result<GlyphRun> {
        attach(
            self.source_sha256.clone(),
            self.apply_ligatures_kerns_with_boundaries(input, boundaries)?,
        )
    }
}
fn attach(tfm_sha256: String, items: Vec<TfmItem>) -> Result<GlyphRun> {
    let mut run = GlyphRun {
        tfm_sha256,
        leading_kern: FixWord(0),
        glyphs: Vec::new(),
    };
    for item in items {
        match item {
            TfmItem::Glyph(g) => run.glyphs.push(KernedGlyph {
                code: g.code,
                input_start: g.input_start,
                input_end: g.input_end,
                kern_after: FixWord(0),
            }),
            TfmItem::Kern(k) => {
                let target = run
                    .glyphs
                    .last_mut()
                    .map(|g| &mut g.kern_after)
                    .unwrap_or(&mut run.leading_kern);
                target.0 = target
                    .0
                    .checked_add(k.0)
                    .ok_or_else(|| invalid("TFM attached kern overflow"))?;
            }
        }
    }
    Ok(run)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn leading_trailing_and_signed_kerns_preserve_exact_values() {
        let g = EncodedGlyph {
            code: 65,
            input_start: 0,
            input_end: 1,
        };
        let r = attach(
            "identity".into(),
            vec![
                TfmItem::Kern(FixWord(-2)),
                TfmItem::Kern(FixWord(1)),
                TfmItem::Glyph(g),
                TfmItem::Kern(FixWord(8)),
                TfmItem::Kern(FixWord(-3)),
            ],
        )
        .unwrap();
        assert_eq!(r.leading_kern, FixWord(-1));
        assert_eq!(r.glyphs[0].kern_after, FixWord(5));
        assert_eq!((r.glyphs[0].input_start, r.glyphs[0].input_end), (0, 1));
        assert_eq!(r.tfm_sha256, "identity");
        assert!(attach(
            "x".into(),
            vec![TfmItem::Kern(FixWord(i32::MAX)), TfmItem::Kern(FixWord(1))]
        )
        .is_err());
        assert!(attach(
            "x".into(),
            vec![
                TfmItem::Glyph(g),
                TfmItem::Kern(FixWord(i32::MIN)),
                TfmItem::Kern(FixWord(-1))
            ]
        )
        .is_err());
    }
}
