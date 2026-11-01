//! Explicit project manifest registry; no discovery scans or system fallback.
mod transport;
use crate::{sha256, FontResource, ManifestEntry};
use flashtex_project_files::{ProjectPath, ProjectRoot, SaveError};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, sync::Arc};
pub use transport::*;
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FontStyle {
    Upright,
    Italic,
    Oblique,
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StyleBinding {
    pub family: String,
    pub weight: u16,
    pub style: FontStyle,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegistryEntry {
    pub binding: StyleBinding,
    pub resource: ManifestEntry,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegistryManifest {
    pub schema_version: u32,
    pub entries: Vec<RegistryEntry>,
}
#[derive(Debug, Clone, Copy)]
pub struct RegistryLimits {
    pub max_entries: usize,
    pub max_files: usize,
    pub max_total_bytes: u64,
    pub max_manifest_bytes: u64,
}
impl Default for RegistryLimits {
    fn default() -> Self {
        Self {
            max_entries: 128,
            max_files: 257,
            max_total_bytes: 256 * 1024 * 1024,
            max_manifest_bytes: 1024 * 1024,
        }
    }
}
#[derive(Debug)]
pub enum RegistryError {
    InvalidManifest(String),
    Missing { path: String },
    MissingBinding(StyleBinding),
    AmbiguousBinding(StyleBinding),
    RootedRead { path: String, error: SaveError },
    ResourceMismatch { path: String, error: crate::Error },
    Budget(&'static str),
    StaleGeneration { expected: String, actual: String },
}
impl std::fmt::Display for RegistryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for RegistryError {}
type Result<T> = std::result::Result<T, RegistryError>;
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiscoveryMetadata {
    pub binding: StyleBinding,
    pub resource: ManifestEntry,
}
/// Frozen snapshot: external file edits never mutate already returned resources.
pub struct ProjectFontRegistry {
    resources: BTreeMap<StyleBinding, RegistryResource>,
    discovery: Vec<DiscoveryMetadata>,
    generation: String,
    loaded_bytes: u64,
    files_read: usize,
}
fn path(raw: &str) -> Result<ProjectPath> {
    if raw.len() > 4096 || raw.split('/').any(|c| c == "..") {
        return Err(RegistryError::InvalidManifest(
            "font paths must not traverse parent directories".into(),
        ));
    }
    let p =
        ProjectPath::normalize(raw).map_err(|e| RegistryError::InvalidManifest(e.to_string()))?;
    if p.as_str() != raw {
        return Err(RegistryError::InvalidManifest(
            "font paths must be canonical project-relative paths".into(),
        ));
    }
    Ok(p)
}
struct Reader<'a> {
    root: &'a ProjectRoot,
    limits: RegistryLimits,
    bytes: u64,
    files: usize,
}
impl Reader<'_> {
    fn read(&mut self, path: &ProjectPath, limit: u64) -> Result<Vec<u8>> {
        if self.files >= self.limits.max_files {
            return Err(RegistryError::Budget("file read count"));
        }
        let remaining = self
            .limits
            .max_total_bytes
            .checked_sub(self.bytes)
            .ok_or(RegistryError::Budget("total bytes"))?;
        if remaining == 0 {
            return Err(RegistryError::Budget("total bytes"));
        }
        self.files += 1;
        let read = self
            .root
            .read(path, limit.min(remaining))
            .map_err(|error| RegistryError::RootedRead {
                path: path.as_str().into(),
                error,
            })?
            .ok_or_else(|| RegistryError::Missing {
                path: path.as_str().into(),
            })?;
        self.bytes += read.bytes.len() as u64;
        Ok(read.bytes)
    }
}
impl ProjectFontRegistry {
    pub fn load(root: &ProjectRoot, manifest_path: &str, limits: RegistryLimits) -> Result<Self> {
        if limits.max_entries > 128
            || limits.max_files > 257
            || limits.max_total_bytes > 256 * 1024 * 1024
            || limits.max_manifest_bytes > 1024 * 1024
        {
            return Err(RegistryError::Budget("configured hard limit"));
        }
        let mut reader = Reader {
            root,
            limits,
            bytes: 0,
            files: 0,
        };
        let raw = reader.read(&path(manifest_path)?, limits.max_manifest_bytes)?;
        let (manifest, claims) = transport::decode(&raw)?;
        if manifest.entries.len() > limits.max_entries {
            return Err(RegistryError::Budget("entry count"));
        }
        let mut ordered = BTreeMap::new();
        let mut ids = BTreeMap::new();
        for entry in manifest.entries {
            if entry.binding.family.trim() != entry.binding.family
                || entry.binding.family.is_empty()
                || entry.binding.family.len() > 256
                || entry.binding.family.chars().any(char::is_control)
                || !(1..=1000).contains(&entry.binding.weight)
            {
                return Err(RegistryError::InvalidManifest(
                    "invalid explicit family/weight".into(),
                ));
            }
            path(&entry.resource.path)?;
            path(&entry.resource.license.text_path)?;
            if let Some(previous) =
                ids.insert(entry.resource.font.font_id.clone(), entry.resource.clone())
            {
                if previous != entry.resource {
                    return Err(RegistryError::InvalidManifest(
                        "resource ID binds different declarations".into(),
                    ));
                }
            }
            let binding = entry.binding.clone();
            if ordered.insert(binding.clone(), entry.resource).is_some() {
                return Err(RegistryError::AmbiguousBinding(binding));
            }
        }
        let mut resources = BTreeMap::new();
        let mut discovery = Vec::new();
        for (binding, entry) in ordered {
            let bytes = reader.read(&path(&entry.path)?, crate::MAX_FONT_BYTES as u64)?;
            let license = reader.read(
                &path(&entry.license.text_path)?,
                crate::MAX_LICENSE_BYTES as u64,
            )?;
            let resource = (match entry.font.format.as_str() {
                "static-cff" => CffFontResource::from_bytes(&entry, &bytes, &license)
                    .map(|v| RegistryResource::Cff(Arc::new(v))),
                _ => FontResource::from_bytes(&entry, &bytes, &license)
                    .map(|v| RegistryResource::TrueType(Arc::new(v))),
            })
            .map_err(|error| RegistryError::ResourceMismatch {
                path: entry.path.clone(),
                error,
            })?;
            resources.insert(binding.clone(), resource);
            discovery.push(DiscoveryMetadata {
                binding,
                resource: entry,
            });
        }
        // Canonical semantic registry generation ignores JSON whitespace/order.
        // Declared full-font and license hashes were verified above.
        let mut canonical = b"flashtex-project-font-registry-v1\0".to_vec();
        canonical.extend(
            serde_json::to_vec(&discovery)
                .map_err(|e| RegistryError::InvalidManifest(e.to_string()))?,
        );
        let registry = Self {
            resources,
            discovery,
            generation: sha256(&canonical),
            loaded_bytes: reader.bytes,
            files_read: reader.files,
        };
        if let Some(claims) = claims {
            registry.validate_import(claims)?;
        }
        Ok(registry)
    }
    pub fn generation(&self) -> &str {
        &self.generation
    }
    pub fn discovery(&self) -> &[DiscoveryMetadata] {
        &self.discovery
    }
    pub fn loaded_bytes(&self) -> u64 {
        self.loaded_bytes
    }
    pub fn files_read(&self) -> usize {
        self.files_read
    }
    /// Existing TrueType-only accessor remains explicit about CFF bindings.
    pub fn get(&self, binding: &StyleBinding) -> Result<Arc<FontResource>> {
        match self.resource(binding)? {
            RegistryResource::TrueType(resource) => Ok(resource),
            RegistryResource::Cff(_) => Err(RegistryError::ResourceMismatch {
                path: binding.family.clone(),
                error: crate::Error::UnsupportedFont(
                    "use typed CFF registry resource accessor".into(),
                ),
            }),
        }
    }
    pub fn resource(&self, binding: &StyleBinding) -> Result<RegistryResource> {
        self.resources
            .get(binding)
            .cloned()
            .ok_or_else(|| RegistryError::MissingBinding(binding.clone()))
    }
    pub fn require_generation(&self, expected: &str) -> Result<()> {
        if expected != self.generation {
            return Err(RegistryError::StaleGeneration {
                expected: expected.into(),
                actual: self.generation.clone(),
            });
        }
        Ok(())
    }
}

/// Distinct backends preserve the production TrueType-only schema boundary.
#[derive(Clone)]
pub enum RegistryResource {
    TrueType(Arc<FontResource>),
    Cff(Arc<CffFontResource>),
}
pub struct CffFontResource {
    descriptor: crate::FontDescriptor,
    license: crate::LicenseMetadata,
    bytes: Arc<[u8]>,
    license_text: Arc<[u8]>,
    identity: crate::cff::CffIdentity,
}
impl CffFontResource {
    fn from_bytes(entry: &ManifestEntry, bytes: &[u8], license: &[u8]) -> crate::Result<Self> {
        use flashtex_font_engine::{Face, TrueTypeFace};
        crate::check_entry_common(entry)?;
        if entry.font.format != "static-cff" || entry.font.face_index != 0 {
            return Err(crate::Error::UnsupportedFont(
                "registry requires static CFF face0".into(),
            ));
        }
        if bytes.len() > crate::MAX_FONT_BYTES || license.len() > crate::MAX_LICENSE_BYTES {
            return Err(crate::Error::SizeLimit);
        }
        if bytes.len() as u64 != entry.font.byte_length {
            return Err(crate::Error::MetadataMismatch("byte_length"));
        }
        if sha256(bytes) != entry.font.sha256 {
            return Err(crate::Error::DigestMismatch);
        }
        if license.is_empty() || sha256(license) != entry.license.text_sha256 {
            return Err(crate::Error::LicenseDigestMismatch);
        }
        // The original reader owns this allocation without moving its contents;
        // its table slice establishes the actual selected byte range.
        let storage = bytes.to_vec();
        let base = storage.as_ptr() as usize;
        let face = TrueTypeFace::parse(storage).map_err(crate::engine_adapter::engine_error)?;
        if face.units_per_em() as u32 != entry.font.units_per_em
            || face.num_glyphs() as u32 != entry.font.glyph_count
            || face.postscript_name() != entry.font.postscript_name
        {
            return Err(crate::Error::MetadataMismatch("CFF metrics/name"));
        }
        let table = face
            .cff_table()
            .ok_or_else(|| crate::invalid("registry declared CFF has no CFF table"))?;
        let start = (table.as_ptr() as usize)
            .checked_sub(base)
            .ok_or_else(|| crate::invalid("CFF accessor allocation range"))?;
        let end = start
            .checked_add(table.len())
            .ok_or_else(|| crate::invalid("CFF accessor range overflow"))?;
        if bytes.get(start..end) != Some(table) {
            return Err(crate::invalid("CFF accessor selected bytes mismatch"));
        }
        let cache = crate::cff::CffOutlineCache::from_font_table(
            bytes,
            0,
            start..end,
            crate::cff::CacheLimits {
                max_entries: 0,
                max_bytes: 0,
            },
        )?;
        if cache.glyph_count() != entry.font.glyph_count as usize {
            return Err(crate::Error::MetadataMismatch("CFF charstring glyph_count"));
        }
        Ok(Self {
            descriptor: entry.font.clone(),
            license: entry.license.clone(),
            bytes: bytes.into(),
            license_text: license.into(),
            identity: cache.identity().clone(),
        })
    }
    pub fn descriptor(&self) -> &crate::FontDescriptor {
        &self.descriptor
    }
    pub fn license(&self) -> &crate::LicenseMetadata {
        &self.license
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn license_text(&self) -> &[u8] {
        &self.license_text
    }
    pub fn identity(&self) -> &crate::cff::CffIdentity {
        &self.identity
    }
    pub fn outline_cache(
        &self,
        limits: crate::cff::CacheLimits,
    ) -> crate::Result<crate::cff::CffOutlineCache> {
        crate::cff::CffOutlineCache::from_font_table(
            &self.bytes,
            self.identity.face_index,
            self.identity.table_range.clone(),
            limits,
        )
    }
    pub fn shape_adapter(&self) -> crate::Result<crate::engine_adapter::EngineFontAdapter> {
        let cache = self.outline_cache(crate::cff::CacheLimits {
            max_entries: 0,
            max_bytes: 0,
        })?;
        crate::engine_adapter::EngineFontAdapter::from_cff(&self.bytes, &cache)
    }
}

/// Explicit rooted VF/TFM dependency declarations.
pub mod vf_project;

pub mod collections;
