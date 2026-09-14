//! Explicit pipeline CFF consumer. This is not a negotiated wire-profile change.
//! Registry resources verify font, face, table and license bytes upstream; this
//! adapter binds their immutable identity to each producer declaration and source.
use crate::{
    batch::{ExactClip, PrimitiveId},
    cubic::CachedCffConsumer,
    mixed::*,
    outlines::{OutlineCoordinate, OutlinePoint},
    *,
};
use flashtex_font_resources::{
    cff::{CacheLimits, HintPolicy},
    registry::CffFontResource,
};
use std::sync::Arc;

pub struct PipelineCff {
    list: DisplayList,
    original: Vec<u8>,
    resources: BTreeMap<String, Arc<CffFontResource>>,
    fonts: BTreeMap<String, CachedCffConsumer>,
}
impl PipelineCff {
    pub fn bind(
        bytes: &[u8],
        capabilities: &Capabilities,
        documents: &BTreeMap<String, SourceSnapshot>,
        resources: &BTreeMap<String, Arc<CffFontResource>>,
    ) -> Result<Self> {
        let Message::DisplayList(list) = parse(bytes)?.message else {
            return Err(ValidationError("display list required".into()));
        };
        Self::bind_list(list, bytes, capabilities, documents, resources)
    }
    /// Consume a private paired proof instead of parsing the same bytes again.
    /// Pairing is not resource authority: all profile/source/font checks below run.
    pub(crate) fn bind_paired(
        paired: pipeline_frame::PairedDisplay<'_>,
        capabilities: &Capabilities,
        documents: &BTreeMap<String, SourceSnapshot>,
        resources: &BTreeMap<String, Arc<CffFontResource>>,
    ) -> Result<Self> {
        let (envelope, bytes) = paired.into_parts();
        let Message::DisplayList(list) = envelope.message else {
            return Err(ValidationError("display list required".into()));
        };
        Self::bind_list(list, bytes, capabilities, documents, resources)
    }
    fn bind_list(
        list: DisplayList,
        bytes: &[u8],
        capabilities: &Capabilities,
        documents: &BTreeMap<String, SourceSnapshot>,
        resources: &BTreeMap<String, Arc<CffFontResource>>,
    ) -> Result<Self> {
        list.validate_profile(capabilities, true)?;
        for d in &list.documents {
            let source = documents
                .get(&d.path)
                .ok_or_else(|| ValidationError("missing source snapshot".into()))?;
            require(
                source.revision == d.revision
                    && source.text.len() as u64 == d.byte_length
                    && digest(source.text.as_bytes()) == d.sha256,
                "source identity mismatch",
            )?;
        }
        // PROPOSAL display-list-v2-window: `fonts[]` is always the whole-document
        // font closure, independent of which pages are currently resident, so
        // binding it here does not need — and must not require — every page to
        // be resident. A frame with only the window's pages resident still
        // binds because this loop never reads page/item residency.
        let mut fonts = BTreeMap::new();
        for f in &list.fonts {
            require(
                f.format == "opentype-cff",
                "pipeline CFF adapter requires explicit CFF resource",
            )?;
            let r = resources
                .get(&f.font_id)
                .ok_or_else(|| ValidationError("missing immutable CFF resource".into()))?;
            let d = r.descriptor();
            require(
                f.sha256 == d.sha256
                    && f.face_index == d.face_index
                    && f.byte_length == d.byte_length
                    && f.units_per_em == d.units_per_em
                    && f.glyph_count == d.glyph_count
                    && f.postscript_name == d.postscript_name,
                "CFF resource metadata mismatch",
            )?;
            let cache = r
                .outline_cache(CacheLimits {
                    max_entries: 64,
                    max_bytes: 8 * 1024 * 1024,
                })
                .map_err(|e| ValidationError(format!("CFF resource: {e:?}")))?;
            fonts.insert(f.font_id.clone(), CachedCffConsumer::new(cache));
        }
        // PROPOSAL display-list-v2-window: an elided page's `items` is always
        // empty, so this loop naturally contributes nothing for it — no
        // special-casing needed to bind a windowed frame.
        for page in &list.pages {
            for item in &page.items {
                match item {
                    Item::GlyphRun(run) => {
                        for c in &run.clusters {
                            if let Some(r) = &c.sources {
                                validate_source_bytes(r, documents)?;
                            }
                        }
                    }
                    Item::Rule(rule) => {
                        if let Some(r) = &rule.sources {
                            validate_source_bytes(r, documents)?;
                        }
                    }
                }
            }
        }
        for d in &list.diagnostics {
            validate_source_bytes(&d.sources, documents)?;
        }
        Ok(Self {
            list,
            fonts,
            original: bytes.to_vec(),
            resources: resources.clone(),
        })
    }
    /// Producer advances remain distinct from CFF outline advances; absolute origins
    /// drive painting and the immutable display retains the original metrics.
    pub fn display(&self) -> &DisplayList {
        &self.list
    }
    pub fn page(
        &self,
        index: usize,
        policy: HintPolicy,
        limits: MixedLimits,
    ) -> MixedResult<MixedBatch> {
        let p = self.list.pages.get(index).ok_or(MixedError::Identity)?;
        let clip = ExactClip::from_rect(&HitRect {
            x: Tick(0),
            top: Tick(0),
            width: p.width,
            height: p.height,
        })?;
        let mut primitives = Vec::new();
        let mut commands = 0usize;
        let q = |t: Tick| OutlineCoordinate::from_fraction(t.0 as i128, 1);
        for (item_index, item) in p.items.iter().enumerate() {
            match item {
                Item::GlyphRun(run) => {
                    for (glyph_index, g) in run.glyphs.iter().enumerate() {
                        if primitives.len() >= limits.max_primitives {
                            return Err(MixedError::Budget);
                        }
                        let c = &run.clusters[g.cluster as usize];
                        let outline = self.fonts[&run.font_id]
                            .place_cached(
                                g.gid as u16,
                                policy,
                                q(run.font_size)?,
                                OutlinePoint {
                                    x: q(g.origin_x)?,
                                    y: q(g.baseline_y)?,
                                },
                                limits.max_commands.saturating_sub(commands),
                            )?
                            .outline;
                        commands = commands
                            .checked_add(outline.commands.len())
                            .ok_or(MixedError::Budget)?;
                        primitives.push(MixedPrimitive {
                            identity: PrimitiveId {
                                item_index,
                                glyph_index: Some(glyph_index),
                            },
                            geometry: MixedGeometry::Cubic(Box::new(outline)),
                            clip,
                            paint: run.paint.clone(),
                            source_chain: Vec::new(),
                            sources: c.sources.clone().unwrap_or_default(),
                            synthetic_reason: c.synthetic_reason.clone(),
                            logical_interval: Some((c.text_start_byte, c.text_end_byte)),
                            font_sha256: Some(
                                self.fonts[&run.font_id].identity().font_sha256.clone(),
                            ),
                            original_gid: Some(g.gid),
                        });
                    }
                }
                Item::Rule(r) => {
                    if primitives.len() >= limits.max_primitives {
                        return Err(MixedError::Budget);
                    }
                    primitives.push(MixedPrimitive {
                        identity: PrimitiveId {
                            item_index,
                            glyph_index: None,
                        },
                        geometry: MixedGeometry::Rule(ExactClip::from_rect(&HitRect {
                            x: r.x,
                            top: r.top,
                            width: r.width,
                            height: r.height,
                        })?),
                        clip,
                        paint: r.paint.clone(),
                        source_chain: Vec::new(),
                        sources: r.sources.clone().unwrap_or_default(),
                        synthetic_reason: r.synthetic_reason.clone(),
                        logical_interval: None,
                        font_sha256: None,
                        original_gid: None,
                    });
                }
            }
        }
        MixedBatch::assembled(
            MixedContext {
                project_id: &self.list.project_id,
                revision: self.list.revision,
                page: p.number,
                page_width: p.width,
                page_height: p.height,
                clip,
            },
            primitives,
            limits,
        )
    }
}

/// Searchable PDF emitted by the existing PDF owner, after immutable input gates.
/// ToUnicode is exact only for the accepted one-glyph-per-cluster profile.
pub struct SearchablePdf {
    pub bytes: Vec<u8>,
    pub input_sha256: String,
    pub report: flashtex_pdf::v2::V2Report,
}
impl PipelineCff {
    pub fn export_searchable(&self, max_pdf_bytes: usize) -> Result<SearchablePdf> {
        require(
            !self
                .list
                .diagnostics
                .iter()
                .any(|d| matches!(d.severity, Severity::Error)),
            "error diagnostics prevent searchable export",
        )?;
        require(
            max_pdf_bytes > 0 && max_pdf_bytes <= 64 * 1024 * 1024,
            "PDF output budget",
        )?;
        require(self.list.pages.len() <= 256, "PDF page budget")?;
        let mut mappings: BTreeMap<(&str, u32), &str> = BTreeMap::new();
        let mut glyphs = 0usize;
        for page in &self.list.pages {
            for item in &page.items {
                if let Item::GlyphRun(run) = item {
                    glyphs = glyphs
                        .checked_add(run.glyphs.len())
                        .ok_or_else(|| ValidationError("glyph budget".into()))?;
                    require(glyphs <= 100000, "glyph budget")?;
                    let mut counts = vec![0usize; run.clusters.len()];
                    for g in &run.glyphs {
                        counts[g.cluster as usize] += 1;
                    }
                    require(
                        counts.iter().all(|n| *n == 1),
                        "multi-glyph cluster requires ActualText support",
                    )?;
                    for g in &run.glyphs {
                        let c = &run.clusters[g.cluster as usize];
                        let text = &run.text[c.text_start_byte as usize..c.text_end_byte as usize];
                        require(
                            !text.is_empty(),
                            "empty cluster requires ActualText support",
                        )?;
                        if let Some(previous) = mappings.insert((&run.font_id, g.gid), text) {
                            require(
                                previous == text,
                                "ambiguous GID text requires ActualText support",
                            )?;
                        }
                    }
                }
            }
        }
        let directory = tempfile::tempdir()
            .map_err(|e| ValidationError(format!("private font staging: {e}")))?;
        let mut total = 0usize;
        let mut expected = BTreeMap::new();
        for f in &self.list.fonts {
            let bytes = self.resources[&f.font_id].bytes();
            total = total
                .checked_add(bytes.len())
                .ok_or_else(|| ValidationError("font byte budget".into()))?;
            require(total <= 64 * 1024 * 1024, "font byte budget")?;
            let path = directory.path().join(format!("{}.otf", f.sha256));
            std::fs::write(&path, bytes)
                .map_err(|e| ValidationError(format!("font staging: {e}")))?;
            expected.insert(f.font_id.as_str(), path);
        }
        let input = std::str::from_utf8(&self.original)
            .map_err(|_| ValidationError("UTF8 envelope".into()))?;
        let (document, report) = flashtex_pdf::v2::from_v2(
            input,
            &flashtex_pdf::v2::V2Options {
                font_dirs: vec![directory.path().to_path_buf()],
            },
        )
        .map_err(ValidationError)?;
        for font in &report.fonts {
            require(
                expected.get(font.font_id.as_str()) == Some(&font.path),
                "unexpected font resolver path",
            )?;
            require(
                font.hash_form == flashtex_pdf::v2::HashForm::Bytes,
                "raw font identity required",
            )?;
        }
        let output = flashtex_pdf::exact::render_exact(&document)
            .map_err(|e| ValidationError(format!("PDF export: {e:?}")))?;
        require(output.bytes.len() <= max_pdf_bytes, "PDF output budget")?;
        require(
            output.warnings.is_empty(),
            "PDF export warnings require review",
        )?;
        Ok(SearchablePdf {
            bytes: output.bytes,
            input_sha256: digest(&self.original),
            report,
        })
    }
}
