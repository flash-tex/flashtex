//! Explicit immutable project registry -> render leases. No implicit family,
//! fallback, global font lookup, native activation or device-grid policy.
use crate::{
    cubic::CachedCffConsumer,
    glyph_cache::GlyphPathCache,
    shaped_replay::{ReplayLimits, ShapedReplay},
    shaped_run::{OutlineSource, PlacedShapedRun, Placement, PlacementLimits},
    *,
};
use flashtex_font_resources::{
    cff::{CacheLimits, HintPolicy},
    engine_adapter::{BoundShapedRun, EngineFontAdapter, ShapeRequest},
    registry::{ProjectFontRegistry, RegistryResource, StyleBinding},
    ManifestEntry,
};
use serde::Serialize;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, Mutex,
};
static NEXT_INSTANCE: AtomicU64 = AtomicU64::new(1);
#[derive(Debug)]
pub enum BindingError {
    Invalid(ValidationError),
    Registry(flashtex_font_resources::registry::RegistryError),
    Font(flashtex_font_resources::Error),
    Placement(crate::shaped_run::ShapePlacementError),
    StaleLease,
    Budget,
    CacheUnavailable,
}
impl From<ValidationError> for BindingError {
    fn from(v: ValidationError) -> Self {
        Self::Invalid(v)
    }
}
type BoundResult<T> = std::result::Result<T, BindingError>;
#[derive(Debug, Clone, Copy)]
pub struct RegistryRenderLimits {
    pub max_bindings: usize,
    pub max_cache_bytes: usize,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CffBindingIdentity {
    pub sha256: String,
    pub offset: usize,
    pub byte_length: usize,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ResourceBinding {
    pub project_instance: String,
    pub generation: String,
    pub selection: StyleBinding,
    pub declaration: ManifestEntry,
    pub cff_table: Option<CffBindingIdentity>,
}
enum Backend {
    TrueType {
        resource: Arc<flashtex_font_resources::FontResource>,
        cache: Mutex<GlyphPathCache>,
    },
    Cff {
        resource: CachedCffConsumer,
    },
}
struct BoundResource {
    binding: ResourceBinding,
    engine: EngineFontAdapter,
    backend: Backend,
}
#[derive(Clone)]
pub struct RenderLease {
    instance: u64,
    epoch: u64,
    resource: Arc<BoundResource>,
}
impl RenderLease {
    pub fn binding(&self) -> &ResourceBinding {
        &self.resource.binding
    }
}
#[derive(Clone)]
pub struct RegistryFrame {
    retained_resource: Arc<BoundResource>,
    binding: ResourceBinding,
    run: Arc<PlacedShapedRun>,
}
impl RegistryFrame {
    pub fn binding(&self) -> &ResourceBinding {
        &self.retained_resource.binding
    }
    pub fn run(&self) -> &PlacedShapedRun {
        &self.run
    }
    pub fn replay_bytes(&self, max_bytes: usize) -> BoundResult<Vec<u8>> {
        let shape = self.run.replay_bytes(max_bytes)?;
        let value = crate::mixed_replay::parse_unique(&shape)
            .map_err(|e| ValidationError(e.to_string()))?;
        let mut output = crate::mixed::BoundedOutput {
            bytes: vec![],
            limit: max_bytes.min(MAX_MESSAGE_BYTES),
        };
        serde_json::to_writer(&mut output,&serde_json::json!({"format":"flashtex-internal-registry-shaped-v1","binding":self.binding,"shape":value})).map_err(|_|BindingError::Budget)?;
        Ok(output.bytes)
    }
}
pub struct RegistryRenderer {
    project_instance: String,
    instance: u64,
    epoch: u64,
    registry: Arc<ProjectFontRegistry>,
    resources: BTreeMap<StyleBinding, Arc<BoundResource>>,
    limits: RegistryRenderLimits,
    invalidations: u64,
}
impl RegistryRenderer {
    pub fn new(
        project_instance: &str,
        registry: Arc<ProjectFontRegistry>,
        limits: RegistryRenderLimits,
    ) -> BoundResult<Self> {
        id(project_instance)?;
        if !(1..=128).contains(&limits.max_bindings)
            || limits.max_cache_bytes < limits.max_bindings * 256
            || limits.max_cache_bytes > 64 * 1024 * 1024
        {
            return Err(BindingError::Budget);
        }
        let instance = NEXT_INSTANCE
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |v| v.checked_add(1))
            .map_err(|_| BindingError::Budget)?;
        Ok(Self {
            project_instance: project_instance.into(),
            instance,
            epoch: 0,
            registry,
            resources: BTreeMap::new(),
            limits,
            invalidations: 0,
        })
    }
    pub fn generation(&self) -> &str {
        self.registry.generation()
    }
    pub fn cached_bindings(&self) -> usize {
        self.resources.len()
    }
    pub fn invalidations(&self) -> u64 {
        self.invalidations
    }
    /// Same semantic generation retains caches. A->B->A cannot revive an old lease.
    pub fn replace(&mut self, registry: Arc<ProjectFontRegistry>) -> BoundResult<bool> {
        if registry.generation() == self.generation() {
            return Ok(false);
        }
        self.epoch = self.epoch.checked_add(1).ok_or(BindingError::Budget)?;
        self.invalidations = self
            .invalidations
            .checked_add(1)
            .ok_or(BindingError::Budget)?;
        self.resources.clear();
        self.registry = registry;
        Ok(true)
    }
    pub fn bind(
        &mut self,
        selection: &StyleBinding,
        expected_generation: &str,
    ) -> BoundResult<RenderLease> {
        self.registry
            .require_generation(expected_generation)
            .map_err(BindingError::Registry)?;
        if let Some(resource) = self.resources.get(selection) {
            return Ok(RenderLease {
                instance: self.instance,
                epoch: self.epoch,
                resource: resource.clone(),
            });
        }
        if self.resources.len() >= self.limits.max_bindings {
            return Err(BindingError::Budget);
        }
        let declaration = self
            .registry
            .discovery()
            .iter()
            .find(|d| &d.binding == selection)
            .ok_or_else(|| {
                BindingError::Registry(
                    flashtex_font_resources::registry::RegistryError::MissingBinding(
                        selection.clone(),
                    ),
                )
            })?
            .resource
            .clone();
        let resource = self
            .registry
            .resource(selection)
            .map_err(BindingError::Registry)?;
        let capacity = self.limits.max_cache_bytes / self.limits.max_bindings;
        let (engine, backend, cff_table) = match resource {
            RegistryResource::TrueType(resource) => {
                require(
                    resource.descriptor() == &declaration.font
                        && resource.license() == &declaration.license,
                    "registry TrueType declaration mismatch",
                )?;
                let engine =
                    EngineFontAdapter::from_resource(&resource).map_err(BindingError::Font)?;
                (
                    engine,
                    Backend::TrueType {
                        resource,
                        cache: Mutex::new(GlyphPathCache::new(256, capacity)?),
                    },
                    None,
                )
            }
            RegistryResource::Cff(resource) => {
                require(
                    resource.descriptor() == &declaration.font
                        && resource.license() == &declaration.license,
                    "registry CFF declaration mismatch",
                )?;
                let identity = resource.identity();
                let table = CffBindingIdentity {
                    sha256: identity.cff_sha256.clone(),
                    offset: identity.table_range.start,
                    byte_length: identity.table_range.len(),
                };
                let engine = resource.shape_adapter().map_err(BindingError::Font)?;
                let cache = resource
                    .outline_cache(CacheLimits {
                        max_entries: 256,
                        max_bytes: capacity,
                    })
                    .map_err(BindingError::Font)?;
                (
                    engine,
                    Backend::Cff {
                        resource: CachedCffConsumer::new(cache),
                    },
                    Some(table),
                )
            }
        };
        let binding = ResourceBinding {
            project_instance: self.project_instance.clone(),
            generation: self.generation().into(),
            selection: selection.clone(),
            declaration,
            cff_table,
        };
        let resource = Arc::new(BoundResource {
            binding,
            engine,
            backend,
        });
        self.resources.insert(selection.clone(), resource.clone());
        Ok(RenderLease {
            instance: self.instance,
            epoch: self.epoch,
            resource,
        })
    }
    fn current(&self, lease: &RenderLease) -> BoundResult<()> {
        if lease.instance != self.instance
            || lease.epoch != self.epoch
            || lease.binding().generation != self.generation()
            || !self
                .resources
                .get(&lease.binding().selection)
                .is_some_and(|r| Arc::ptr_eq(r, &lease.resource))
        {
            return Err(BindingError::StaleLease);
        }
        Ok(())
    }
    pub fn shape(
        &self,
        lease: &RenderLease,
        request: ShapeRequest<'_>,
    ) -> BoundResult<BoundShapedRun> {
        self.current(lease)?;
        lease
            .resource
            .engine
            .shape(request)
            .map_err(BindingError::Font)
    }
    pub fn place(
        &self,
        lease: &RenderLease,
        run: &BoundShapedRun,
        snapshot: &SourceSnapshot,
        placement: Placement,
        limits: PlacementLimits,
        cff_policy: HintPolicy,
    ) -> BoundResult<RegistryFrame> {
        self.current(lease)?;
        require(
            run.identity() == lease.resource.engine.identity(),
            "registry shaped engine identity mismatch",
        )?;
        let result = match &lease.resource.backend {
            Backend::TrueType { resource, cache } => {
                let mut cache = cache.lock().map_err(|_| BindingError::CacheUnavailable)?;
                PlacedShapedRun::prepare(
                    run,
                    snapshot,
                    OutlineSource::TrueType {
                        resource,
                        cache: &mut cache,
                    },
                    placement,
                    limits,
                )
            }
            Backend::Cff { resource } => PlacedShapedRun::prepare(
                run,
                snapshot,
                OutlineSource::Cff {
                    resource,
                    policy: cff_policy,
                },
                placement,
                limits,
            ),
        }
        .map_err(BindingError::Placement)?;
        Ok(RegistryFrame {
            retained_resource: lease.resource.clone(),
            binding: lease.binding().clone(),
            run: Arc::new(result),
        })
    }
    /// Verify evidence against this current immutable registry lease and source.
    /// This still does not prove external commands equal the actual font outlines.
    pub fn verify_replay(
        &self,
        lease: &RenderLease,
        bytes: &[u8],
        source_path: &str,
        snapshot: &SourceSnapshot,
    ) -> BoundResult<ShapedReplay> {
        self.current(lease)?;
        require(bytes.len() <= MAX_MESSAGE_BYTES, "registry replay byte cap")?;
        let value =
            crate::mixed_replay::parse_unique(bytes).map_err(|e| ValidationError(e.to_string()))?;
        require(
            value.as_object().is_some_and(|o| o.len() == 3)
                && value["format"] == "flashtex-internal-registry-shaped-v1",
            "registry replay fields/format",
        )?;
        require(
            value["binding"]
                == serde_json::to_value(lease.binding())
                    .map_err(|e| ValidationError(e.to_string()))?,
            "registry replay binding identity",
        )?;
        let shape =
            serde_json::to_vec(&value["shape"]).map_err(|e| ValidationError(e.to_string()))?;
        let replay = ShapedReplay::parse(&shape, ReplayLimits::default())?;
        replay.verify_source(source_path, snapshot)?;
        let identity = lease.resource.engine.identity();
        require(
            replay.metadata()["identity"]["font_sha256"] == identity.font_sha256
                && replay.metadata()["identity"]["engine_font_id"]
                    == identity.engine_font_id.content_hex()
                && replay.metadata()["identity"]["face_index"] == identity.face_index,
            "registry replay shaping identity",
        )?;
        for p in replay.metadata()["primitives"]
            .as_array()
            .expect("validated primitive array")
        {
            match &lease.binding().cff_table {
                None => require(p["kind"] == "quadratic", "registry replay backend")?,
                Some(table) => require(
                    p["kind"] == "cubic"
                        && p["cff"]["resource"]["cff_sha256"] == table.sha256
                        && p["cff"]["resource"]["table_range"]
                            == serde_json::json!([table.offset, table.offset + table.byte_length]),
                    "registry replay CFF table identity",
                )?,
            }
        }
        Ok(replay)
    }
}

pub mod selection;

impl RegistryRenderer {
    /// Deterministic version2 registry manifest; caller owns any rooted save.
    /// Export pins the same generation used by rendering leases.
    pub fn export_manifest(
        &self,
        expected_generation: &str,
        max_bytes: usize,
    ) -> BoundResult<Vec<u8>> {
        self.registry
            .require_generation(expected_generation)
            .map_err(BindingError::Registry)?;
        self.registry
            .export_json(max_bytes)
            .map_err(BindingError::Registry)
    }
    /// Explicit bounded font discovery for UI consumers, never an implicit choice.
    pub fn metadata_page(
        &self,
        expected_generation: &str,
        filter: flashtex_font_resources::registry::MetadataFilter<'_>,
        offset: usize,
        limit: usize,
    ) -> BoundResult<flashtex_font_resources::registry::MetadataPage> {
        self.registry
            .enumerate(expected_generation, filter, offset, limit)
            .map_err(BindingError::Registry)
    }
}

pub mod nested;

pub mod math;
