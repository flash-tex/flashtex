//! Original font-engine consumer, not another parser/shaper. Integer font-unit
//! output stays exact; TFM encoding and page/device conversion are separate.
use crate::{invalid, sha256, Error, FontResource, Result};
use flashtex_font_engine::{Face, FontId, ShapeOptions, Shaped, TrueTypeFace};
use std::ops::Range;

pub const ENGINE_SOURCE_SHA256: &str = env!("FLASHTEX_FONT_ENGINE_SOURCE_SHA256");
pub const ENGINE_PEER_REVISION: &str = "f418238899d1218b17cff9a33601790a3057949a";
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShapeIdentity {
    pub font_sha256: String,
    pub face_index: u32,
    /// Engine hashes full font bytes followed by big-endian face index.
    pub engine_font_id: FontId,
}
pub struct EngineFontAdapter {
    face: TrueTypeFace,
    identity: ShapeIdentity,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputEncoding {
    Unicode,
    TexEightBit,
}
pub struct ShapeRequest<'a> {
    pub source: &'a str,
    pub source_sha256: &'a str,
    pub path: &'a str,
    pub revision: u64,
    pub range: Range<usize>,
    pub font_sha256: &'a str,
    pub face_index: u32,
    pub encoding: InputEncoding,
    /// Any requested variation instance is explicitly unsupported.
    pub variation_coordinates: &'a [([u8; 4], i32)],
    pub options: ShapeOptions,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShapeSource {
    pub path: String,
    pub revision: u64,
    pub source_sha256: String,
    pub range: Range<usize>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundShapedRun {
    identity: ShapeIdentity,
    source: ShapeSource,
    shaped: Shaped,
    cache_key: String,
    options: ShapeOptions,
}
impl BoundShapedRun {
    pub fn identity(&self) -> &ShapeIdentity {
        &self.identity
    }
    pub fn source(&self) -> &ShapeSource {
        &self.source
    }
    /// Clusters retain range relative to source.range.start. No GID renumbering.
    pub fn shaped(&self) -> &Shaped {
        &self.shaped
    }
    pub fn options(&self) -> ShapeOptions {
        self.options
    }
    pub fn cache_key(&self) -> &str {
        &self.cache_key
    }
    pub fn absolute_cluster_range(&self, index: usize) -> Result<Range<usize>> {
        let range = &self
            .shaped
            .clusters
            .get(index)
            .ok_or_else(|| invalid("shape cluster index"))?
            .source_range;
        Ok(self.source.range.start + range.start..self.source.range.start + range.end)
    }
}
pub(crate) fn engine_error(error: flashtex_font_engine::Error) -> Error {
    match error {
        flashtex_font_engine::Error::Unsupported(_)
        | flashtex_font_engine::Error::UnsupportedFeature { .. }
        | flashtex_font_engine::Error::UnsupportedScript { .. } => {
            Error::UnsupportedFont(error.to_string())
        }
        _ => invalid(&format!("original font engine: {error}")),
    }
}
impl EngineFontAdapter {
    pub fn from_resource(resource: &FontResource) -> Result<Self> {
        let adapter = Self::from_verified_bytes(
            resource.bytes(),
            &resource.descriptor().sha256,
            resource.descriptor().face_index,
        )?;
        if adapter.face.units_per_em() as u32 != resource.descriptor().units_per_em
            || adapter.face.num_glyphs() as u32 != resource.descriptor().glyph_count
        {
            return Err(invalid("font-engine resource metadata mismatch"));
        }
        Ok(adapter)
    }
    /// Existing CFF cache establishes full-font/table identity. This reuses the
    /// peer OpenType parser and verifies its selected CFF table against the cache.
    pub fn from_cff(bytes: &[u8], cache: &crate::cff::CffOutlineCache) -> Result<Self> {
        let identity = cache.identity();
        let adapter = Self::from_verified_bytes(bytes, &identity.font_sha256, identity.face_index)?;
        let table = adapter
            .face
            .cff_table()
            .ok_or_else(|| invalid("font-engine CFF table missing"))?;
        if sha256(table) != identity.cff_sha256
            || bytes.get(identity.table_range.clone()) != Some(table)
        {
            return Err(invalid("font-engine CFF table identity mismatch"));
        }
        Ok(adapter)
    }
    fn from_verified_bytes(bytes: &[u8], expected: &str, face_index: u32) -> Result<Self> {
        if bytes.len() > crate::MAX_FONT_BYTES {
            return Err(Error::SizeLimit);
        }
        if sha256(bytes) != expected {
            return Err(Error::DigestMismatch);
        }
        if face_index != 0 {
            return Err(Error::UnsupportedFont(
                "adapter static single-face profile".into(),
            ));
        }
        let face = TrueTypeFace::parse(bytes.to_vec()).map_err(engine_error)?;
        let mut engine_input = bytes.to_vec();
        engine_input.extend(face_index.to_be_bytes());
        if face.id().content_hex() != sha256(&engine_input) {
            return Err(invalid("font-engine face identity convention mismatch"));
        }
        let identity = ShapeIdentity {
            font_sha256: expected.into(),
            face_index,
            engine_font_id: face.id().clone(),
        };
        Ok(Self { face, identity })
    }
    pub(crate) fn face(&self) -> &TrueTypeFace {
        &self.face
    }
    pub fn identity(&self) -> &ShapeIdentity {
        &self.identity
    }
    pub fn units_per_em(&self) -> u16 {
        self.face.units_per_em()
    }
    pub fn shape(&self, request: ShapeRequest<'_>) -> Result<BoundShapedRun> {
        if request.font_sha256 != self.identity.font_sha256
            || request.face_index != self.identity.face_index
        {
            return Err(invalid("shape request font identity mismatch"));
        }
        if request.encoding != InputEncoding::Unicode || !request.variation_coordinates.is_empty() {
            return Err(Error::UnsupportedFont("shape adapter requires Unicode and static variation context; TeX encoding stays separate".into()));
        }
        if request.source.len() > 1024 * 1024
            || request.path.len() > 4096
            || request.range.len() > 65536
        {
            return Err(Error::SizeLimit);
        }
        if sha256(request.source.as_bytes()) != request.source_sha256 {
            return Err(Error::DigestMismatch);
        }
        let text = request
            .source
            .get(request.range.clone())
            .ok_or_else(|| invalid("shape source range/UTF8 boundary"))?;
        if text
            .chars()
            .any(|c| matches!(c as u32,0xFE00..=0xFE0F|0xE0100..=0xE01EF))
        {
            return Err(Error::UnsupportedFont(
                "variation selector context unsupported by adapter".into(),
            ));
        }
        if !request.options.fail_on_unsupported_lookups
            && (request.options.kerning
                || request.options.ligatures
                || request.options.compose_marks)
        {
            return Err(Error::UnsupportedFont(
                "shape adapter requires strict requested-feature handling".into(),
            ));
        }
        let shaped = flashtex_font_engine::shape::shape(&self.face, text, &request.options)
            .map_err(engine_error)?;
        if shaped.fonts.as_slice() != [self.identity.engine_font_id.clone()]
            || shaped.units_per_em != self.face.units_per_em()
        {
            return Err(invalid("shape output resource mismatch/fallback"));
        }
        if !shaped.missing.is_empty() {
            return Err(Error::UnsupportedFont(format!(
                "shape missing glyphs: {:?}",
                shaped.missing
            )));
        }
        let mut end = 0;
        for cluster in &shaped.clusters {
            if cluster.font != 0
                || cluster.source_range.start != end
                || text.get(cluster.source_range.clone()) != Some(cluster.text.as_str())
                || cluster
                    .glyphs
                    .iter()
                    .any(|g| g.gid.0 >= self.face.num_glyphs())
            {
                return Err(invalid("shape output original GID/source cluster mismatch"));
            }
            end = cluster.source_range.end;
        }
        if end != text.len() {
            return Err(invalid("shape output incomplete source coverage"));
        }
        let source = ShapeSource {
            path: request.path.into(),
            revision: request.revision,
            source_sha256: request.source_sha256.into(),
            range: request.range,
        };
        let mut key = Vec::new();
        for part in [
            "font-engine-adapter-v1",
            ENGINE_SOURCE_SHA256,
            &sha256(include_bytes!("engine_adapter.rs")),
            &self.identity.font_sha256,
            &self.identity.engine_font_id.content_hex(),
            &source.path,
            &source.source_sha256,
        ] {
            key.extend((part.len() as u64).to_be_bytes());
            key.extend(part.as_bytes());
        }
        for number in [
            source.revision,
            source.range.start as u64,
            source.range.end as u64,
            self.identity.face_index as u64,
        ] {
            key.extend(number.to_be_bytes());
        }
        key.extend([
            request.options.ligatures as u8,
            request.options.kerning as u8,
            request.options.compose_marks as u8,
            request.options.cmap_ligature_fallback as u8,
            request.options.fail_on_unsupported_lookups as u8,
        ]);
        Ok(BoundShapedRun {
            identity: self.identity.clone(),
            source,
            shaped,
            cache_key: sha256(&key),
            options: request.options,
        })
    }
}
