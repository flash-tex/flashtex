//! Immutable revision/resource-aware display cache. Only the currently requested
//! generation can install a frame; font or source identity changes invalidate old
//! tickets even when filenames and the project revision are unchanged.
use crate::{hit_test::PageIndex, *};
use std::io::Write;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct RenderIdentity {
    project_id: String,
    revision: u64,
    configuration_sha256: String,
    fingerprint: String,
}
impl RenderIdentity {
    /// Build from authoritative source/font manifests selected before rendering.
    /// The configuration digest must cover compiler/layout options and versions.
    pub fn new(
        project_id: String,
        revision: u64,
        configuration_sha256: String,
        mut documents: Vec<DocumentResource>,
        mut fonts: Vec<FontResource>,
    ) -> Result<Self> {
        hash(&configuration_sha256)?;
        documents.sort_by(|a, b| a.path.cmp(&b.path));
        fonts.sort_by(|a, b| a.font_id.cmp(&b.font_id));
        let shell = DisplayList {
            render_format: RenderFormat::DisplayListV2,
            coordinate_unit: "bp_2pow20".into(),
            color_space: "srgb".into(),
            text_extraction: "cluster-actualtext".into(),
            project_id: project_id.clone(),
            revision,
            required_features: vec![Feature::RgbaSrgb, Feature::ClusterActualText],
            documents,
            fonts,
            pages: vec![],
            diagnostics: vec![],
            window: None,
        };
        shell.validate(&Capabilities {
            render_formats: vec![RenderFormat::DisplayListV2],
            features: vec![Feature::RgbaSrgb, Feature::ClusterActualText],
        })?;
        let encoded = encoded(&shell)?;
        let mut hasher = Sha256::new();
        hasher.update(configuration_sha256.as_bytes());
        hasher.update(encoded);
        Ok(Self {
            project_id,
            revision,
            configuration_sha256,
            fingerprint: format!("{:x}", hasher.finalize()),
        })
    }
    pub fn project_id(&self) -> &str {
        &self.project_id
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
}
#[derive(Debug, Clone)]
pub struct RenderTicket {
    identity: RenderIdentity,
    generation: u64,
}
impl RenderTicket {
    pub fn revision(&self) -> u64 {
        self.identity.revision
    }
    pub fn project_id(&self) -> &str {
        &self.identity.project_id
    }
}
#[derive(Debug)]
pub struct CachedDisplay {
    display: DisplayList,
    index: PageIndex,
    fingerprint: String,
    retained_bytes: usize,
    font_bytes: BTreeMap<String, Arc<[u8]>>,
}
impl CachedDisplay {
    pub fn display(&self) -> &DisplayList {
        &self.display
    }
    pub fn index(&self) -> &PageIndex {
        &self.index
    }
    pub fn font_bytes(&self, font_id: &str) -> Option<&[u8]> {
        self.font_bytes.get(font_id).map(|v| v.as_ref())
    }
    /// Cache identity does not establish renderer acceptance/outline painting.
    pub fn paintable(&self) -> bool {
        false
    }
}
struct Project {
    identity: RenderIdentity,
    generation: u64,
    display: Option<Arc<CachedDisplay>>,
}
pub struct DisplayCache {
    projects: BTreeMap<String, Project>,
    max_projects: usize,
    max_bytes: usize,
    retained_bytes: usize,
}
impl DisplayCache {
    pub fn new(max_projects: usize, max_bytes: usize) -> Result<Self> {
        require(
            (1..=4096).contains(&max_projects) && max_bytes > 0,
            "invalid cache capacity",
        )?;
        Ok(Self {
            projects: BTreeMap::new(),
            max_projects,
            max_bytes,
            retained_bytes: 0,
        })
    }
    /// Returns the same ticket for identical inputs. Resource/configuration
    /// changes replace the current expectation and immediately remove its frame.
    pub fn begin(&mut self, identity: RenderIdentity) -> Result<RenderTicket> {
        if let Some(project) = self.projects.get_mut(&identity.project_id) {
            require(
                identity.revision >= project.identity.revision,
                "cannot request an older revision",
            )?;
            if identity.fingerprint != project.identity.fingerprint {
                let generation = project
                    .generation
                    .checked_add(1)
                    .ok_or_else(|| ValidationError("cache generation overflow".into()))?;
                if let Some(old) = project.display.take() {
                    self.retained_bytes -= old.retained_bytes;
                }
                project.identity = identity.clone();
                project.generation = generation;
            }
            return Ok(RenderTicket {
                identity: project.identity.clone(),
                generation: project.generation,
            });
        }
        require(
            self.projects.len() < self.max_projects,
            "cache project capacity reached",
        )?;
        let ticket = RenderTicket {
            identity: identity.clone(),
            generation: 1,
        };
        self.projects.insert(
            identity.project_id.clone(),
            Project {
                identity,
                generation: 1,
                display: None,
            },
        );
        Ok(ticket)
    }
    /// Explicit invalidation preserves the revision high-water mark and blocks
    /// already-issued jobs. Caller-held old Arcs remain immutable, not current.
    pub fn invalidate(&mut self, project_id: &str) -> Result<()> {
        let project = self
            .projects
            .get_mut(project_id)
            .ok_or_else(|| ValidationError("unknown cache project".into()))?;
        project.generation = project
            .generation
            .checked_add(1)
            .ok_or_else(|| ValidationError("cache generation overflow".into()))?;
        if let Some(old) = project.display.take() {
            self.retained_bytes -= old.retained_bytes;
        }
        Ok(())
    }
    pub fn current(&self, ticket: &RenderTicket) -> Result<Option<Arc<CachedDisplay>>> {
        Ok(self.check_ticket(ticket)?.display.clone())
    }
    fn check_ticket(&self, ticket: &RenderTicket) -> Result<&Project> {
        let project = self
            .projects
            .get(&ticket.identity.project_id)
            .ok_or_else(|| ValidationError("unknown cache project".into()))?;
        require(
            project.generation == ticket.generation
                && project.identity.fingerprint == ticket.identity.fingerprint,
            "stale render ticket",
        )?;
        Ok(project)
    }
    /// Install one immutable complete list, never a mixture of page revisions.
    /// Verifies exact source/font resources before accepting the current frame.
    pub fn install<V: FontValidator>(
        &mut self,
        ticket: &RenderTicket,
        display: DisplayList,
        capabilities: &Capabilities,
        documents: &BTreeMap<String, SourceSnapshot>,
        fonts: &BTreeMap<String, Vec<u8>>,
        verifier: &V,
    ) -> Result<Arc<CachedDisplay>> {
        self.check_ticket(ticket)?;
        display.validate(capabilities)?;
        let identity = RenderIdentity::new(
            display.project_id.clone(),
            display.revision,
            ticket.identity.configuration_sha256.clone(),
            display.documents.clone(),
            display.fonts.clone(),
        )?;
        require(
            identity.fingerprint == ticket.identity.fingerprint,
            "display does not match requested resources/revision",
        )?;
        let data = encoded(&display)?;
        let fingerprint = digest(&data);
        if let Some(current) = &self.check_ticket(ticket)?.display {
            require(
                current.fingerprint == fingerprint,
                "conflicting output for immutable render identity",
            )?;
            return Ok(Arc::clone(current));
        }
        display.validate_resources(capabilities, documents, fonts, verifier)?;
        let size = display.fonts.iter().try_fold(data.len(), |size, font| {
            size.checked_add(font.byte_length as usize)
                .ok_or_else(|| ValidationError("cache size overflow".into()))
        })?;
        require(
            size <= self.max_bytes.saturating_sub(self.retained_bytes),
            "cache byte capacity reached",
        )?;
        let index = PageIndex::build(&display, capabilities)?;
        let font_bytes = display
            .fonts
            .iter()
            .map(|font| {
                (
                    font.font_id.clone(),
                    Arc::from(fonts[&font.font_id].as_slice()),
                )
            })
            .collect();
        let entry = Arc::new(CachedDisplay {
            display,
            index,
            fingerprint,
            retained_bytes: size,
            font_bytes,
        });
        let project = self
            .projects
            .get_mut(&ticket.identity.project_id)
            .expect("ticket checked under exclusive borrow");
        project.display = Some(Arc::clone(&entry));
        self.retained_bytes += size;
        Ok(entry)
    }
    /// Accounts retained serialized display bytes and font bytes. Index/allocator
    /// overhead and externally held Arcs are additional, not a total RSS claim.
    pub fn retained_payload_bytes(&self) -> usize {
        self.retained_bytes
    }
}
struct BoundedBytes(Vec<u8>);
impl Write for BoundedBytes {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if bytes.len() > MAX_MESSAGE_BYTES.saturating_sub(self.0.len()) {
            return Err(std::io::Error::other("display exceeds 32 MiB"));
        }
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
fn encoded<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    let mut output = BoundedBytes(Vec::new());
    serde_json::to_writer(&mut output, value)
        .map_err(|e| ValidationError(format!("bounded display encoding: {e}")))?;
    Ok(output.0)
}
