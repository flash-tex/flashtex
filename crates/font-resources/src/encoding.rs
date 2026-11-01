//! Explicit declared encoding bindings; no Unicode casting, glyph-name guessing or fallback.
use crate::{
    invalid,
    tfm::{CharacterMetrics, FixWord, Tfm, TfmItem},
    FontResource, Result,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EncodingEntry {
    pub code: u8,
    pub glyph_name: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NamedGlyph {
    pub glyph_name: String,
    pub glyph_id: u16,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EncodingManifest {
    pub tfm_sha256: String,
    pub font_sha256: String,
    pub face_index: u32,
    pub encoding: Vec<EncodingEntry>,
    pub declared_glyphs: Vec<NamedGlyph>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlyphIdentity {
    Original(u16),
    Notdef,
}
#[derive(Debug, Clone)]
pub struct EncodingMap {
    font_sha256: String,
    tfm_sha256: String,
    face_index: u32,
    mapping: BTreeMap<u8, (String, GlyphIdentity)>,
}
fn name_valid(name: &str) -> bool {
    !name.is_empty() && name.len() <= 256 && !name.chars().any(char::is_control)
}
impl EncodingMap {
    /// Names/GIDs are explicit caller declarations, not inferred PostScript name-table claims.
    pub fn bind(manifest: &EncodingManifest, font: &FontResource, tfm: &Tfm) -> Result<Self> {
        if manifest.font_sha256 != font.descriptor().sha256
            || manifest.face_index != font.descriptor().face_index
            || manifest.tfm_sha256 != tfm.source_sha256
        {
            return Err(invalid("encoding resource identity mismatch"));
        }
        if manifest.encoding.len() > 256 || manifest.declared_glyphs.len() > 65536 {
            return Err(invalid("encoding declaration budget"));
        }
        let mut names = BTreeMap::new();
        for entry in &manifest.declared_glyphs {
            if !name_valid(&entry.glyph_name)
                || entry.glyph_id as u32 >= font.descriptor().glyph_count
            {
                return Err(invalid("invalid declared glyph name/GID"));
            }
            if entry.glyph_name == ".notdef" && entry.glyph_id != 0 {
                return Err(invalid(".notdef must denote glyph zero"));
            }
            if names
                .insert(entry.glyph_name.clone(), entry.glyph_id)
                .is_some()
            {
                return Err(invalid("duplicate declared glyph name"));
            }
        }
        let mut mapping = BTreeMap::new();
        for entry in &manifest.encoding {
            if !name_valid(&entry.glyph_name) {
                return Err(invalid("invalid encoding glyph name"));
            }
            let id = if entry.glyph_name == ".notdef" {
                GlyphIdentity::Notdef
            } else {
                let gid = *names
                    .get(&entry.glyph_name)
                    .ok_or_else(|| invalid("encoding glyph name has no declared font mapping"))?;
                if gid == 0 {
                    return Err(invalid(
                        "named glyph maps to .notdef; explicit .notdef required",
                    ));
                }
                GlyphIdentity::Original(gid)
            };
            if mapping
                .insert(entry.code, (entry.glyph_name.clone(), id))
                .is_some()
            {
                return Err(invalid("duplicate encoding character code"));
            }
        }
        Ok(Self {
            font_sha256: manifest.font_sha256.clone(),
            tfm_sha256: manifest.tfm_sha256.clone(),
            face_index: manifest.face_index,
            mapping,
        })
    }
    pub fn resolve(&self, code: u8) -> Result<GlyphIdentity> {
        self.mapping
            .get(&code)
            .map(|(_, id)| *id)
            .ok_or_else(|| invalid("TFM code absent from explicit encoding"))
    }
    pub fn glyph_name(&self, code: u8) -> Option<&str> {
        self.mapping.get(&code).map(|(name, _)| name.as_str())
    }
    pub fn font_sha256(&self) -> &str {
        &self.font_sha256
    }
    pub fn tfm_sha256(&self) -> &str {
        &self.tfm_sha256
    }
    pub fn face_index(&self) -> u32 {
        self.face_index
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MappedItem {
    Glyph {
        tfm_code: u8,
        identity: GlyphIdentity,
        metrics: CharacterMetrics,
        input_start: usize,
        input_end: usize,
    },
    Kern(FixWord),
}
/// Borrowed immutable TFM/font identity plus a verified explicit encoding declaration.
pub struct BoundTfmFont<'a> {
    tfm: &'a Tfm,
    font: &'a FontResource,
    encoding: EncodingMap,
}
impl<'a> BoundTfmFont<'a> {
    pub fn new(tfm: &'a Tfm, font: &'a FontResource, manifest: &EncodingManifest) -> Result<Self> {
        Ok(Self {
            tfm,
            font,
            encoding: EncodingMap::bind(manifest, font, tfm)?,
        })
    }
    pub fn encoding(&self) -> &EncodingMap {
        &self.encoding
    }
    pub fn font(&self) -> &'a FontResource {
        self.font
    }
    pub fn tfm(&self) -> &'a Tfm {
        self.tfm
    }
    pub fn design_size(&self) -> FixWord {
        self.tfm.design_size
    }
    pub fn map_code(&self, code: u8) -> Result<(GlyphIdentity, CharacterMetrics)> {
        Ok((
            self.encoding.resolve(code)?,
            self.tfm
                .char_metrics(code)
                .ok_or_else(|| invalid("encoded TFM character has no metrics"))?,
        ))
    }
    pub fn map_run(&self, input: &[u8]) -> Result<Vec<MappedItem>> {
        self.map_run_with_boundaries(input, crate::tfm::BoundaryOptions::default())
    }
    pub fn map_run_with_boundaries(
        &self,
        input: &[u8],
        boundaries: crate::tfm::BoundaryOptions,
    ) -> Result<Vec<MappedItem>> {
        self.tfm
            .apply_ligatures_kerns_with_boundaries(input, boundaries)?
            .into_iter()
            .map(|item| match item {
                TfmItem::Kern(kern) => Ok(MappedItem::Kern(kern)),
                TfmItem::Glyph(glyph) => {
                    let (identity, metrics) = self.map_code(glyph.code)?;
                    Ok(MappedItem::Glyph {
                        tfm_code: glyph.code,
                        identity,
                        metrics,
                        input_start: glyph.input_start,
                        input_end: glyph.input_end,
                    })
                }
            })
            .collect()
    }
}
