//! Explicit collection registry consumer. No collection-directory parser is
//! implemented here. A trusted complete-layout resolver is REQUIRED: the peer
//! font engine selects faces but does not expose validated layout enumeration.
use super::{path, Reader, RegistryLimits, RegistryManifest, StyleBinding};
use crate::{sha256, Error, ManifestEntry, Result};
use flashtex_font_engine::{Face, FontSource, Outlines, TrueTypeFace};
use flashtex_project_files::ProjectRoot;
use std::{collections::BTreeMap, ops::Range, sync::Arc};
#[derive(Debug, Clone)]
pub struct TableRange {
    pub tag: [u8; 4],
    pub range: Range<usize>,
}
#[derive(Debug, Clone)]
pub struct FaceLayout {
    pub directory: Range<usize>,
    pub tables: Vec<TableRange>,
}
#[derive(Debug, Clone)]
pub struct CollectionLayout {
    pub header: Range<usize>,
    pub faces: Vec<FaceLayout>,
}
/// Trusted parser boundary: enumerate EVERY face/directory/table, rejecting
/// unsupported TTC headers, duplicate directory records and malformed offsets.
/// No built-in resolver is supplied until the original engine exposes this API.
pub trait VerifiedCollectionResolver {
    fn implementation_identity(&self) -> &str;
    fn complete_layout(&self, bytes: &[u8]) -> Result<CollectionLayout>;
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableIdentity {
    pub tag: [u8; 4],
    pub range: Range<usize>,
    pub sha256: String,
}
pub struct CollectionResource {
    declaration: ManifestEntry,
    face: TrueTypeFace,
    license: Arc<[u8]>,
    tables: Vec<TableIdentity>,
    resolver_identity: String,
}
impl CollectionResource {
    pub fn declaration(&self) -> &ManifestEntry {
        &self.declaration
    }
    pub fn face(&self) -> &TrueTypeFace {
        &self.face
    }
    pub fn bytes(&self) -> &[u8] {
        self.face.program()
    }
    pub fn license_text(&self) -> &[u8] {
        &self.license
    }
    pub fn tables(&self) -> &[TableIdentity] {
        &self.tables
    }
    pub fn resolver_identity(&self) -> &str {
        &self.resolver_identity
    }
}
fn valid_range(r: &Range<usize>, len: usize) -> bool {
    r.start <= r.end && r.end <= len
}
fn intersects(a: &Range<usize>, b: &Range<usize>) -> bool {
    a.start < b.end && b.start < a.end
}
fn validate_layout(layout: &CollectionLayout, len: usize) -> Result<()> {
    if layout.faces.is_empty()
        || layout.faces.len() > 128
        || layout.header.start != 0
        || layout.header.end < 12
        || !valid_range(&layout.header, len)
    {
        return Err(crate::invalid("collection layout/header bounds"));
    }
    let mut directories = vec![layout.header.clone()];
    let mut tables = Vec::new();
    for (index, face) in layout.faces.iter().enumerate() {
        if face.tables.is_empty()
            || face.tables.len() > 256
            || !valid_range(&face.directory, len)
            || face.directory.len() < 12 + face.tables.len() * 16
        {
            return Err(crate::invalid("collection directory bounds"));
        }
        directories.push(face.directory.clone());
        let mut tags = BTreeMap::new();
        for t in &face.tables {
            if !valid_range(&t.range, len)
                || !t.range.start.is_multiple_of(4)
                || tags.insert(t.tag, ()).is_some()
            {
                return Err(crate::invalid(
                    "collection table range/alignment/duplicate tag",
                ));
            }
            if !t.range.is_empty() {
                tables.push((t.range.clone(), t.tag, index));
            }
        }
    }
    directories.sort_by_key(|r| r.start);
    if directories.windows(2).any(|w| intersects(&w[0], &w[1])) {
        return Err(crate::invalid("collection directory overlap"));
    }
    tables.sort_by_key(|(r, _, _)| (r.start, r.end));
    for (r, _, _) in &tables {
        if directories.iter().any(|d| intersects(r, d)) {
            return Err(crate::invalid("collection table overlaps directory"));
        }
    }
    for pair in tables.windows(2) {
        let (a, tag_a, face_a) = &pair[0];
        let (b, tag_b, face_b) = &pair[1];
        if intersects(a, b) && (a != b || tag_a != tag_b || face_a == face_b) {
            return Err(crate::invalid("collection table overlap"));
        }
    }
    Ok(())
}
pub struct CollectionRegistry {
    generation: String,
    resources: BTreeMap<StyleBinding, Arc<CollectionResource>>,
}
impl CollectionRegistry {
    /// Separate opt-in registry; does not change rendering-v2's single-face ABI.
    pub fn load(
        root: &ProjectRoot,
        manifest_path: &str,
        limits: RegistryLimits,
        resolver: &dyn VerifiedCollectionResolver,
    ) -> std::result::Result<Self, super::RegistryError> {
        use super::RegistryError as E;
        if limits.max_entries > 128
            || limits.max_files > 257
            || limits.max_total_bytes > 256 * 1024 * 1024
            || limits.max_manifest_bytes > 1024 * 1024
        {
            return Err(E::Budget("collection registry hard cap"));
        }
        let resolver_id = resolver.implementation_identity();
        if resolver_id.is_empty() || resolver_id.len() > 256 {
            return Err(E::InvalidManifest(
                "explicit resolver identity required".into(),
            ));
        }
        let mut reader = Reader {
            root,
            limits,
            bytes: 0,
            files: 0,
        };
        let raw = reader.read(&path(manifest_path)?, limits.max_manifest_bytes)?;
        let manifest: RegistryManifest =
            serde_json::from_slice(&raw).map_err(|e| E::InvalidManifest(e.to_string()))?;
        if manifest.schema_version != 1 || manifest.entries.len() > limits.max_entries {
            return Err(E::Budget("collection manifest version/entries"));
        }
        let mut resources = BTreeMap::new();
        let mut canonical = BTreeMap::new();
        for entry in manifest.entries {
            if entry.binding.family.is_empty()
                || entry.binding.family.len() > 256
                || entry.binding.family.trim() != entry.binding.family
                || entry.binding.family.chars().any(char::is_control)
                || !(1..=1000).contains(&entry.binding.weight)
            {
                return Err(E::InvalidManifest("collection style binding".into()));
            }
            if resources.contains_key(&entry.binding) {
                return Err(E::AmbiguousBinding(entry.binding));
            }
            let bytes = reader.read(&path(&entry.resource.path)?, crate::MAX_FONT_BYTES as u64)?;
            let license = reader.read(
                &path(&entry.resource.license.text_path)?,
                crate::MAX_LICENSE_BYTES as u64,
            )?;
            let resource = build(&entry.resource, bytes, license, resolver).map_err(|error| {
                E::ResourceMismatch {
                    path: entry.resource.path.clone(),
                    error,
                }
            })?;
            canonical.insert(entry.binding.clone(), entry.resource);
            resources.insert(entry.binding, Arc::new(resource));
        }
        let mut generation = b"flashtex-explicit-collection-registry-v1\0".to_vec();
        generation.extend(resolver_id.as_bytes());
        for (binding, entry) in canonical {
            generation.extend(
                serde_json::to_vec(&(binding, entry))
                    .map_err(|e| E::InvalidManifest(e.to_string()))?,
            );
        }
        Ok(Self {
            generation: sha256(&generation),
            resources,
        })
    }
    pub fn generation(&self) -> &str {
        &self.generation
    }
    pub fn get(
        &self,
        binding: &StyleBinding,
        expected_generation: &str,
    ) -> std::result::Result<Arc<CollectionResource>, super::RegistryError> {
        if expected_generation != self.generation {
            return Err(super::RegistryError::StaleGeneration {
                expected: expected_generation.into(),
                actual: self.generation.clone(),
            });
        }
        self.resources
            .get(binding)
            .cloned()
            .ok_or_else(|| super::RegistryError::MissingBinding(binding.clone()))
    }
}
fn build(
    entry: &ManifestEntry,
    bytes: Vec<u8>,
    license: Vec<u8>,
    resolver: &dyn VerifiedCollectionResolver,
) -> Result<CollectionResource> {
    crate::check_entry_common(entry)?;
    if bytes.len() > crate::MAX_FONT_BYTES
        || bytes.len() as u64 != entry.font.byte_length
        || sha256(&bytes) != entry.font.sha256
    {
        return Err(Error::DigestMismatch);
    }
    if license.is_empty() || sha256(&license) != entry.license.text_sha256 {
        return Err(Error::LicenseDigestMismatch);
    }
    if !bytes.starts_with(b"ttcf") {
        return Err(crate::invalid("explicit collection requires TTC program"));
    }
    let layout = resolver.complete_layout(&bytes)?;
    validate_layout(&layout, bytes.len())?;
    let selected = layout
        .faces
        .get(entry.font.face_index as usize)
        .ok_or_else(|| crate::invalid("explicit collection face out of bounds"))?;
    let face = TrueTypeFace::parse_with_source(
        bytes,
        FontSource::Memory {
            face_index: entry.font.face_index,
        },
    )
    .map_err(crate::engine_adapter::engine_error)?;
    let expected_format = match face.outlines() {
        Outlines::Glyf => "collection-truetype",
        Outlines::Cff => "collection-cff",
    };
    if entry.font.format != expected_format
        || face.units_per_em() as u32 != entry.font.units_per_em
        || face.num_glyphs() as u32 != entry.font.glyph_count
        || face.postscript_name() != entry.font.postscript_name
    {
        return Err(Error::MetadataMismatch(
            "collection selected-face descriptor",
        ));
    }
    let mut tables = Vec::new();
    for table in &selected.tables {
        let actual = face
            .table(&table.tag)
            .ok_or_else(|| crate::invalid("resolver table absent from selected peer face"))?;
        let start = actual.as_ptr() as usize - face.program().as_ptr() as usize;
        if (start..start + actual.len()) != table.range {
            return Err(crate::invalid(
                "resolver table range differs from peer face",
            ));
        }
        tables.push(TableIdentity {
            tag: table.tag,
            range: table.range.clone(),
            sha256: sha256(actual),
        });
    }
    let mut expected_id = face.program().to_vec();
    expected_id.extend(entry.font.face_index.to_be_bytes());
    if face.id().content_hex() != sha256(&expected_id) {
        return Err(crate::invalid("collection engine identity mismatch"));
    }
    Ok(CollectionResource {
        declaration: entry.clone(),
        face,
        license: license.into(),
        tables,
        resolver_identity: resolver.implementation_identity().into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shared_tables_require_identical_tag_range_and_distinct_faces() {
        let mut layout = CollectionLayout {
            header: 0..16,
            faces: vec![
                FaceLayout {
                    directory: 16..44,
                    tables: vec![TableRange {
                        tag: *b"head",
                        range: 72..80,
                    }],
                },
                FaceLayout {
                    directory: 44..72,
                    tables: vec![TableRange {
                        tag: *b"head",
                        range: 72..80,
                    }],
                },
            ],
        };
        assert!(validate_layout(&layout, 96).is_ok());
        layout.faces[1].tables[0].range = 76..84;
        assert!(validate_layout(&layout, 96).is_err());
        layout.faces[1].tables[0].range = 72..80;
        layout.faces[1].tables[0].tag = *b"maxp";
        assert!(validate_layout(&layout, 96).is_err());
        layout.faces[1].tables[0].range = 0..8;
        assert!(validate_layout(&layout, 96).is_err());
        layout.faces[1].tables[0].range = 100..104;
        assert!(validate_layout(&layout, 96).is_err());
    }
}

#[cfg(test)]
mod contract_tests {
    use super::*;
    fn layout() -> CollectionLayout {
        CollectionLayout {
            header: 0..16,
            faces: vec![FaceLayout {
                directory: 32..60,
                tables: vec![TableRange {
                    tag: *b"cmap",
                    range: 96..104,
                }],
            }],
        }
    }
    #[test]
    fn complete_descriptor_must_keep_tables_out_of_header_and_directories() {
        assert!(validate_layout(&layout(), 128).is_ok());
        for range in [0..8, 32..40] {
            let mut value = layout();
            value.faces[0].tables[0].range = range;
            assert!(validate_layout(&value, 128).is_err());
        }
        let mut value = layout();
        value.faces[0].directory = 8..36;
        assert!(validate_layout(&value, 128).is_err());
    }
    #[test]
    fn bounded_contract_and_same_tag_exact_sharing_are_required() {
        let mut value = layout();
        value.faces.push(FaceLayout {
            directory: 64..92,
            tables: value.faces[0].tables.clone(),
        });
        assert!(validate_layout(&value, 128).is_ok());
        value.faces[1].tables[0].tag = *b"head";
        assert!(validate_layout(&value, 128).is_err());
        value = layout();
        value.faces = vec![value.faces[0].clone(); 129];
        assert!(validate_layout(&value, 128).is_err());
        value = layout();
        value.faces[0].tables = vec![value.faces[0].tables[0].clone(); 257];
        assert!(validate_layout(&value, 128).is_err());
    }
}
