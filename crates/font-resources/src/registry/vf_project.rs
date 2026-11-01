//! Rooted, explicitly licensed VF dependency resolution. No special handlers.
use super::*;
use crate::{
    encoding::{BoundTfmFont, EncodingManifest},
    tfm::Tfm,
    vf::VirtualFont,
    vf_graph::{NestedPacket, Resource, ResourceGraph, ResourceKey},
};
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Asset {
    pub path: String,
    pub sha256: String,
    pub license: crate::LicenseMetadata,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalFont {
    pub id: i32,
    pub target: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Node {
    Physical {
        id: String,
        tfm: Asset,
        binding: StyleBinding,
        encoding: EncodingManifest,
    },
    CffPhysical {
        id: String,
        tfm: Asset,
        binding: StyleBinding,
        encoding: crate::cff::CffEncodingManifest,
    },
    /// Schema2: literal encoding file supplies all256 slots.
    PhysicalEncodingAsset {
        id: String,
        tfm: Asset,
        binding: StyleBinding,
        encoding_asset: Asset,
        declared_glyphs: Vec<crate::encoding::NamedGlyph>,
    },
    CffPhysicalEncodingAsset {
        id: String,
        tfm: Asset,
        binding: StyleBinding,
        encoding_asset: Asset,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        declarations: Option<crate::enc_file::CffMappingDeclarations>,
    },
    Virtual {
        id: String,
        tfm: Asset,
        vf: Asset,
        fonts: Vec<LocalFont>,
    },
}
impl Node {
    fn id(&self) -> &str {
        match self {
            Self::Physical { id, .. }
            | Self::CffPhysical { id, .. }
            | Self::PhysicalEncodingAsset { id, .. }
            | Self::CffPhysicalEncodingAsset { id, .. }
            | Self::Virtual { id, .. } => id,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DependencyManifest {
    pub schema_version: u32,
    pub registry_generation: String,
    pub root: String,
    pub nodes: Vec<Node>,
}
enum Loaded {
    Physical {
        tfm: Tfm,
        font: Arc<FontResource>,
        encoding: EncodingManifest,
    },
    CffPhysical {
        tfm: Tfm,
        font: Arc<CffFontResource>,
        encoding: Arc<crate::cff::ResolvedCffEncoding>,
    },
    Virtual {
        tfm: Tfm,
        vf: VirtualFont,
        fonts: BTreeMap<i32, String>,
    },
}
impl Loaded {
    fn tfm(&self) -> &Tfm {
        match self {
            Self::Physical { tfm, .. }
            | Self::CffPhysical { tfm, .. }
            | Self::Virtual { tfm, .. } => tfm,
        }
    }
    fn key(&self) -> ResourceKey {
        match self {
            Self::Physical { tfm, font, .. } => ResourceKey::Physical {
                font_sha256: font.descriptor().sha256.clone(),
                tfm_sha256: tfm.source_sha256.clone(),
                face_index: font.descriptor().face_index,
            },
            Self::CffPhysical {
                tfm,
                font,
                encoding,
            } => ResourceKey::CffPhysical {
                font_sha256: font.identity().font_sha256.clone(),
                cff_sha256: font.identity().cff_sha256.clone(),
                tfm_sha256: tfm.source_sha256.clone(),
                encoding_sha256: encoding.encoding_sha256().into(),
                face_index: font.identity().face_index,
            },
            Self::Virtual { tfm, vf, .. } => ResourceKey::Virtual {
                vf_sha256: vf.source_sha256.clone(),
                tfm_sha256: tfm.source_sha256.clone(),
            },
        }
    }
}
pub struct ResolvedVfProject {
    manifest: DependencyManifest,
    nodes: BTreeMap<String, Loaded>,
    generation: String,
    license_texts: Vec<Arc<[u8]>>,
    loaded_bytes: u64,
}
fn resource_error(path: &str, error: crate::Error) -> RegistryError {
    RegistryError::ResourceMismatch {
        path: path.into(),
        error,
    }
}
fn asset(
    reader: &mut Reader<'_>,
    decl: &Asset,
    limit: u64,
    licenses: &mut Vec<Arc<[u8]>>,
) -> Result<Vec<u8>> {
    if !crate::valid_hash(&decl.sha256) {
        return Err(RegistryError::InvalidManifest("VF asset SHA256".into()));
    }
    crate::check_license_metadata(&decl.license).map_err(|e| resource_error(&decl.path, e))?;
    let license = reader.read(
        &path(&decl.license.text_path)?,
        crate::MAX_LICENSE_BYTES as u64,
    )?;
    if license.is_empty() || sha256(&license) != decl.license.text_sha256 {
        return Err(resource_error(
            &decl.path,
            crate::Error::LicenseDigestMismatch,
        ));
    }
    let bytes = reader.read(&path(&decl.path)?, limit)?;
    if sha256(&bytes) != decl.sha256 {
        return Err(resource_error(&decl.path, crate::Error::DigestMismatch));
    }
    licenses.push(license.into());
    Ok(bytes)
}
impl ResolvedVfProject {
    pub fn load(
        root: &ProjectRoot,
        manifest_path: &str,
        registry: &ProjectFontRegistry,
        limits: RegistryLimits,
    ) -> Result<Self> {
        if limits.max_entries > 128
            || limits.max_files > 257
            || limits.max_manifest_bytes > 1024 * 1024
            || limits.max_total_bytes > 256 * 1024 * 1024
        {
            return Err(RegistryError::Budget("VF dependency configured bounds"));
        }
        let mut reader = Reader {
            root,
            limits,
            bytes: 0,
            files: 0,
        };
        let raw = reader.read(&path(manifest_path)?, limits.max_manifest_bytes)?;
        let mut manifest: DependencyManifest = serde_json::from_slice(&raw)
            .map_err(|e| RegistryError::InvalidManifest(e.to_string()))?;
        if !matches!(manifest.schema_version, 1 | 2)
            || !crate::valid_hash(&manifest.registry_generation)
        {
            return Err(RegistryError::InvalidManifest(
                "VF dependency schema/generation".into(),
            ));
        }
        registry.require_generation(&manifest.registry_generation)?;
        if manifest.nodes.len() > limits.max_entries {
            return Err(RegistryError::Budget("VF dependency nodes"));
        }
        manifest.nodes.sort_by(|a, b| a.id().cmp(b.id()));
        let mut nodes = BTreeMap::new();
        let mut license_texts = Vec::new();
        for node in &mut manifest.nodes {
            let id = node.id().to_owned();
            if id.is_empty()
                || id.len() > 128
                || !id
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"_.-".contains(&b))
                || nodes.contains_key(&id)
            {
                return Err(RegistryError::InvalidManifest(
                    "VF node ID duplicate/invalid".into(),
                ));
            }
            let mut prepared = match node {
                Node::PhysicalEncodingAsset {
                    tfm,
                    binding,
                    encoding_asset,
                    declared_glyphs,
                    ..
                } => {
                    if manifest.schema_version != 2 {
                        return Err(RegistryError::InvalidManifest(
                            "encoding assets require schema2".into(),
                        ));
                    }
                    if declared_glyphs.len() > 65536 {
                        return Err(RegistryError::Budget("declared glyphs"));
                    }
                    let bytes = asset(
                        &mut reader,
                        encoding_asset,
                        crate::enc_file::MAX_ENC_BYTES as u64,
                        &mut license_texts,
                    )?;
                    let enc = crate::enc_file::EncFile::parse(&bytes, &encoding_asset.sha256)
                        .map_err(|e| RegistryError::InvalidManifest(e.to_string()))?;
                    let font = registry.get(binding)?;
                    Some(Node::Physical {
                        id: id.clone(),
                        tfm: tfm.clone(),
                        binding: binding.clone(),
                        encoding: EncodingManifest {
                            tfm_sha256: tfm.sha256.clone(),
                            font_sha256: font.descriptor().sha256.clone(),
                            face_index: font.descriptor().face_index,
                            encoding: enc.slots().to_vec(),
                            declared_glyphs: declared_glyphs.clone(),
                        },
                    })
                }
                _ => None,
            };
            let loaded = match prepared.as_mut().unwrap_or(node) {
                Node::Physical {
                    tfm,
                    binding,
                    encoding,
                    ..
                } => {
                    let bytes = asset(&mut reader, tfm, 131068, &mut license_texts)?;
                    let parsed = Tfm::parse(&bytes).map_err(|e| resource_error(&tfm.path, e))?;
                    let font = registry.get(binding)?;
                    BoundTfmFont::new(&parsed, &font, encoding)
                        .map_err(|e| resource_error(&tfm.path, e))?;
                    Loaded::Physical {
                        tfm: parsed,
                        font,
                        encoding: encoding.clone(),
                    }
                }
                Node::CffPhysical {
                    tfm,
                    binding,
                    encoding,
                    ..
                } => {
                    let bytes = asset(&mut reader, tfm, 131068, &mut license_texts)?;
                    let parsed = Tfm::parse(&bytes).map_err(|e| resource_error(&tfm.path, e))?;
                    let RegistryResource::Cff(font) = registry.resource(binding)? else {
                        return Err(RegistryError::InvalidManifest(
                            "CFF VF endpoint selected non-CFF registry resource".into(),
                        ));
                    };
                    let cache = font
                        .outline_cache(crate::cff::CacheLimits {
                            max_entries: 0,
                            max_bytes: 0,
                        })
                        .map_err(|e| resource_error(&tfm.path, e))?;
                    let bound = crate::cff::BoundCffTfmFont::new(&parsed, &cache, encoding)
                        .map_err(|e| resource_error(&tfm.path, e))?;
                    let resolved = bound.encoding().clone();
                    Loaded::CffPhysical {
                        tfm: parsed,
                        font,
                        encoding: resolved,
                    }
                }
                Node::CffPhysicalEncodingAsset {
                    tfm,
                    binding,
                    encoding_asset,
                    declarations,
                    ..
                } => {
                    if manifest.schema_version != 2 {
                        return Err(RegistryError::InvalidManifest(
                            "encoding assets require schema2".into(),
                        ));
                    }
                    let encbytes = asset(
                        &mut reader,
                        encoding_asset,
                        crate::enc_file::MAX_ENC_BYTES as u64,
                        &mut license_texts,
                    )?;
                    let enc = crate::enc_file::EncFile::parse(&encbytes, &encoding_asset.sha256)
                        .map_err(|e| RegistryError::InvalidManifest(e.to_string()))?;
                    let tfmbytes = asset(&mut reader, tfm, 131068, &mut license_texts)?;
                    let parsed = Tfm::parse(&tfmbytes).map_err(|e| resource_error(&tfm.path, e))?;
                    let RegistryResource::Cff(font) = registry.resource(binding)? else {
                        return Err(RegistryError::InvalidManifest(
                            "CFF encoding asset requires CFF resource".into(),
                        ));
                    };
                    let cache = font
                        .outline_cache(crate::cff::CacheLimits {
                            max_entries: 0,
                            max_bytes: 0,
                        })
                        .map_err(|e| resource_error(&tfm.path, e))?;
                    let bound = match declarations {
                        Some(d) => enc.bind_cff_declared(&parsed, &cache, d),
                        None => enc.bind_cff(&parsed, &cache),
                    }
                    .map_err(|e| RegistryError::InvalidManifest(e.to_string()))?;
                    let encoding = bound.font().encoding().clone();
                    Loaded::CffPhysical {
                        tfm: parsed,
                        font,
                        encoding,
                    }
                }
                Node::PhysicalEncodingAsset { .. } => {
                    return Err(RegistryError::InvalidManifest(
                        "unresolved encoding asset".into(),
                    ))
                }
                Node::Virtual { tfm, vf, fonts, .. } => {
                    let bytes = asset(&mut reader, tfm, 131068, &mut license_texts)?;
                    let tfm_parsed =
                        Tfm::parse(&bytes).map_err(|e| resource_error(&tfm.path, e))?;
                    let bytes = asset(&mut reader, vf, 16 * 1024 * 1024, &mut license_texts)?;
                    let vf_parsed =
                        VirtualFont::parse(&bytes).map_err(|e| resource_error(&vf.path, e))?;
                    if vf_parsed.design_size != tfm_parsed.design_size
                        || vf_parsed.checksum != 0
                            && tfm_parsed.checksum != 0
                            && vf_parsed.checksum != tfm_parsed.checksum
                    {
                        return Err(RegistryError::InvalidManifest(
                            "VF/TFM header identity mismatch".into(),
                        ));
                    }
                    if fonts.len() > 4096 {
                        return Err(RegistryError::Budget("VF local fonts"));
                    }
                    fonts.sort_by_key(|f| f.id);
                    let mut edges = BTreeMap::new();
                    for edge in fonts.iter() {
                        if edges.insert(edge.id, edge.target.clone()).is_some() {
                            return Err(RegistryError::InvalidManifest(
                                "duplicate VF local font ID".into(),
                            ));
                        }
                    }
                    if edges.len() != vf_parsed.fonts().len()
                        || edges.keys().ne(vf_parsed.fonts().keys())
                    {
                        return Err(RegistryError::InvalidManifest(
                            "VF local font declarations mismatch".into(),
                        ));
                    }
                    Loaded::Virtual {
                        tfm: tfm_parsed,
                        vf: vf_parsed,
                        fonts: edges,
                    }
                }
            };
            nodes.insert(id, loaded);
        }
        if !nodes.contains_key(&manifest.root) {
            return Err(RegistryError::Missing {
                path: format!("VF root node {}", manifest.root),
            });
        }
        let mut keys = std::collections::BTreeSet::new();
        for node in nodes.values() {
            if !keys.insert(node.key()) {
                return Err(RegistryError::InvalidManifest(
                    "duplicate VF graph resource identity".into(),
                ));
            }
            if let Loaded::Virtual { vf, fonts, .. } = node {
                for (id, target) in fonts {
                    let child = nodes.get(target).ok_or_else(|| RegistryError::Missing {
                        path: format!("VF dependency node {target}"),
                    })?;
                    let def = &vf.fonts()[id];
                    if def.design_size != child.tfm().design_size
                        || def.checksum != 0
                            && child.tfm().checksum != 0
                            && def.checksum != child.tfm().checksum
                    {
                        return Err(RegistryError::InvalidManifest(
                            "VF local TFM checksum/design mismatch".into(),
                        ));
                    }
                }
            }
        }
        fn visit(
            id: &str,
            nodes: &BTreeMap<String, Loaded>,
            active: &mut Vec<String>,
            done: &mut std::collections::BTreeSet<String>,
        ) -> Result<()> {
            if active.iter().any(|s| s == id) || active.len() > 32 {
                return Err(RegistryError::InvalidManifest(
                    "VF dependency cycle/depth".into(),
                ));
            }
            if done.contains(id) {
                return Ok(());
            }
            active.push(id.into());
            if let Loaded::Virtual { fonts, .. } = &nodes[id] {
                for child in fonts.values() {
                    visit(child, nodes, active, done)?;
                }
            }
            active.pop();
            done.insert(id.into());
            Ok(())
        }
        let mut done = std::collections::BTreeSet::new();
        for id in nodes.keys() {
            visit(id, &nodes, &mut Vec::new(), &mut done)?;
        }
        let mut canonical = b"rooted-vf-dependencies-v1\0".to_vec();
        canonical.extend(
            serde_json::to_vec(&manifest)
                .map_err(|e| RegistryError::InvalidManifest(e.to_string()))?,
        );
        Ok(Self {
            manifest,
            nodes,
            generation: sha256(&canonical),
            license_texts,
            loaded_bytes: reader.bytes,
        })
    }
    pub fn generation(&self) -> &str {
        &self.generation
    }
    pub fn manifest(&self) -> &DependencyManifest {
        &self.manifest
    }
    pub fn physical_run(
        &self,
        node_id: &str,
        input: &[u8],
        registry_generation: &str,
    ) -> Result<PhysicalRun> {
        if registry_generation != self.manifest.registry_generation {
            return Err(RegistryError::StaleGeneration {
                expected: self.manifest.registry_generation.clone(),
                actual: registry_generation.into(),
            });
        }
        let node = self
            .nodes
            .get(node_id)
            .ok_or_else(|| RegistryError::Missing {
                path: node_id.into(),
            })?;
        let items = match node {
            Loaded::Physical {
                tfm,
                font,
                encoding,
            } => BoundTfmFont::new(tfm, font, encoding).and_then(|b| b.map_run(input)),
            Loaded::CffPhysical { tfm, encoding, .. } => {
                crate::cff::BoundCffTfmFont::from_resolved(tfm, encoding.clone())
                    .and_then(|b| b.map_run(input))
            }
            Loaded::Virtual { .. } => Err(crate::Error::UnsupportedFont(
                "physical_run requires physical endpoint".into(),
            )),
        }
        .map_err(|e| resource_error(node_id, e))?;
        Ok(PhysicalRun {
            project_generation: self.generation.clone(),
            registry_generation: self.manifest.registry_generation.clone(),
            node_id: node_id.into(),
            items,
        })
    }
    pub fn loaded_bytes(&self) -> u64 {
        self.loaded_bytes
    }
    pub fn license_texts(&self) -> &[Arc<[u8]>] {
        &self.license_texts
    }
    pub fn expand(&self, code: u8, registry_generation: &str) -> Result<NestedPacket> {
        if registry_generation != self.manifest.registry_generation {
            return Err(RegistryError::StaleGeneration {
                expected: self.manifest.registry_generation.clone(),
                actual: registry_generation.into(),
            });
        }
        let mut bindings = Vec::new();
        for node in self.nodes.values() {
            if let Loaded::Physical {
                tfm,
                font,
                encoding,
            } = node
            {
                bindings.push(
                    BoundTfmFont::new(tfm, font, encoding)
                        .map_err(|e| resource_error("VF physical binding", e))?,
                );
            }
        }
        let mut cff_bindings = Vec::new();
        for node in self.nodes.values() {
            if let Loaded::CffPhysical { tfm, encoding, .. } = node {
                cff_bindings.push(
                    crate::cff::BoundCffTfmFont::from_resolved(tfm, encoding.clone())
                        .map_err(|e| resource_error("VF CFF binding", e))?,
                );
            }
        }
        let mut graph = ResourceGraph::new();
        for binding in &cff_bindings {
            graph
                .insert(Resource::CffPhysical(binding))
                .map_err(|e| resource_error("VF CFF graph", e))?;
        }

        for binding in &bindings {
            graph
                .insert(Resource::Physical(binding))
                .map_err(|e| resource_error("VF graph", e))?;
        }
        for node in self.nodes.values() {
            if let Loaded::Virtual { tfm, vf, fonts } = node {
                let fonts = fonts
                    .iter()
                    .map(|(id, target)| (*id, self.nodes[target].key()))
                    .collect();
                graph
                    .insert(Resource::Virtual { tfm, vf, fonts })
                    .map_err(|e| resource_error("VF graph", e))?;
            }
        }
        graph
            .expand(&self.nodes[&self.manifest.root].key(), code)
            .map_err(|e| resource_error("VF expansion", e))
    }
}

/// Original input intervals and exact TFM words; no page-unit conversion.
pub struct PhysicalRun {
    pub project_generation: String,
    pub registry_generation: String,
    pub node_id: String,
    pub items: Vec<crate::encoding::MappedItem>,
}
