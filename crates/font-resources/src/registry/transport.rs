use super::*;
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CffTableDeclaration {
    pub sha256: String,
    pub offset: u64,
    pub byte_length: u64,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExportEntry {
    pub binding: StyleBinding,
    pub resource: ManifestEntry,
    #[serde(default)]
    pub cff_table: Option<CffTableDeclaration>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegistryExport {
    pub schema_version: u32,
    pub generation: String,
    pub entries: Vec<ExportEntry>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ImportDocument {
    schema_version: u32,
    #[serde(default)]
    generation: Option<String>,
    entries: Vec<ExportEntry>,
}
pub(super) fn decode(raw: &[u8]) -> Result<(RegistryManifest, Option<RegistryExport>)> {
    // Deserialize directly into strict structs: no Value/map intermediary that
    // would silently overwrite duplicate JSON object keys.
    let doc: ImportDocument =
        serde_json::from_slice(raw).map_err(|e| RegistryError::InvalidManifest(e.to_string()))?;
    if !matches!(doc.schema_version, 1 | 2) {
        return Err(RegistryError::InvalidManifest(
            "unsupported registry schema".into(),
        ));
    }
    if doc.schema_version == 1
        && (doc.generation.is_some() || doc.entries.iter().any(|e| e.cff_table.is_some()))
    {
        return Err(RegistryError::InvalidManifest(
            "schema1 cannot declare schema2 identity fields".into(),
        ));
    }
    let manifest = RegistryManifest {
        schema_version: 1,
        entries: doc
            .entries
            .iter()
            .map(|e| RegistryEntry {
                binding: e.binding.clone(),
                resource: e.resource.clone(),
            })
            .collect(),
    };
    let claims = if doc.schema_version == 2 {
        let generation = doc
            .generation
            .ok_or_else(|| RegistryError::InvalidManifest("schema2 generation required".into()))?;
        if !crate::valid_hash(&generation) {
            return Err(RegistryError::InvalidManifest(
                "schema2 generation must be canonical SHA256".into(),
            ));
        }
        Some(RegistryExport {
            schema_version: 2,
            generation,
            entries: doc.entries,
        })
    } else {
        None
    };
    Ok((manifest, claims))
}
#[derive(Debug, Clone, Copy, Default)]
pub struct MetadataFilter<'a> {
    pub family_prefix: Option<&'a str>,
    pub weight: Option<u16>,
    pub style: Option<FontStyle>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MetadataPage {
    pub generation: String,
    pub total_matches: usize,
    pub next_offset: Option<usize>,
    pub entries: Vec<ExportEntry>,
}
impl ProjectFontRegistry {
    fn export_entry(&self, entry: &DiscoveryMetadata) -> ExportEntry {
        let cff_table = match self
            .resources
            .get(&entry.binding)
            .expect("validated registry binding")
        {
            RegistryResource::TrueType(_) => None,
            RegistryResource::Cff(font) => Some(CffTableDeclaration {
                sha256: font.identity.cff_sha256.clone(),
                offset: font.identity.table_range.start as u64,
                byte_length: font.identity.table_range.len() as u64,
            }),
        };
        ExportEntry {
            binding: entry.binding.clone(),
            resource: entry.resource.clone(),
            cff_table,
        }
    }
    pub fn export_manifest(&self) -> RegistryExport {
        RegistryExport {
            schema_version: 2,
            generation: self.generation.clone(),
            entries: self
                .discovery
                .iter()
                .map(|e| self.export_entry(e))
                .collect(),
        }
    }
    /// Deterministic compact JSON only; callers choose an explicit rooted save.
    pub fn export_json(&self, max_bytes: usize) -> Result<Vec<u8>> {
        if max_bytes > 1024 * 1024 {
            return Err(RegistryError::Budget("export configured byte limit"));
        }
        struct Limited {
            bytes: Vec<u8>,
            limit: usize,
        }
        impl std::io::Write for Limited {
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                if buf.len() > self.limit - self.bytes.len() {
                    return Err(std::io::Error::other("registry export byte budget"));
                }
                self.bytes.extend_from_slice(buf);
                Ok(buf.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let mut output = Limited {
            bytes: Vec::new(),
            limit: max_bytes,
        };
        serde_json::to_writer(&mut output, &self.export_manifest())
            .map_err(|_| RegistryError::Budget("export bytes"))?;
        Ok(output.bytes)
    }
    pub(super) fn validate_import(&self, claims: RegistryExport) -> Result<()> {
        self.require_generation(&claims.generation)?;
        let mut expected = self.export_manifest().entries;
        let mut actual = claims.entries;
        expected.sort_by(|a, b| a.binding.cmp(&b.binding));
        actual.sort_by(|a, b| a.binding.cmp(&b.binding));
        if actual != expected {
            return Err(RegistryError::InvalidManifest(
                "imported backend/table/resource declarations differ from verified resources"
                    .into(),
            ));
        }
        Ok(())
    }
    /// Bounded exact-prefix filtering, no locale inference or fuzzy discovery.
    pub fn enumerate(
        &self,
        expected_generation: &str,
        filter: MetadataFilter<'_>,
        offset: usize,
        limit: usize,
    ) -> Result<MetadataPage> {
        if expected_generation.len() != 64 {
            return Err(RegistryError::InvalidManifest(
                "enumeration generation length".into(),
            ));
        }
        self.require_generation(expected_generation)?;
        if limit == 0 || limit > 64 || offset > 128 {
            return Err(RegistryError::Budget("metadata page bounds"));
        }
        if filter
            .family_prefix
            .is_some_and(|s| s.len() > 256 || s.chars().any(char::is_control))
            || filter.weight.is_some_and(|w| !(1..=1000).contains(&w))
        {
            return Err(RegistryError::InvalidManifest(
                "metadata filter bounds".into(),
            ));
        }
        let matching = self
            .discovery
            .iter()
            .filter(|e| {
                filter
                    .family_prefix
                    .is_none_or(|s| e.binding.family.starts_with(s))
                    && filter.weight.is_none_or(|w| e.binding.weight == w)
                    && filter.style.is_none_or(|s| e.binding.style == s)
            })
            .collect::<Vec<_>>();
        if offset > matching.len() {
            return Err(RegistryError::InvalidManifest(
                "metadata offset beyond matching entries".into(),
            ));
        }
        let end = (offset + limit).min(matching.len());
        Ok(MetadataPage {
            generation: self.generation.clone(),
            total_matches: matching.len(),
            next_offset: if end < matching.len() {
                Some(end)
            } else {
                None
            },
            entries: matching[offset..end]
                .iter()
                .map(|e| self.export_entry(e))
                .collect(),
        })
    }
}
