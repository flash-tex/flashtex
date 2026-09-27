//! Immutable, digest-bound static TrueType resources. No font discovery/fallback,
//! shaping, rasterization, PDF serialization, or complete outline validation.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fmt,
    fs::File,
    io::Read,
    ops::Range,
    path::{Component, Path},
    sync::Arc,
};

pub const MAX_FONT_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_LICENSE_BYTES: usize = 1024 * 1024;
pub const MAX_COLLECTION_BYTES: u64 = 256 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct FontDescriptor {
    pub font_id: String,
    pub sha256: String,
    pub byte_length: u64,
    pub format: String,
    pub face_index: u32,
    pub units_per_em: u32,
    pub glyph_count: u32,
    pub postscript_name: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EmbeddingPermission {
    Allowed,
    Restricted,
    Unknown,
}
/// Operator-supplied licensing provenance, not an automated legal determination.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct LicenseMetadata {
    pub identifier: String,
    pub copyright: String,
    pub source: String,
    pub text_path: String,
    pub text_sha256: String,
    pub embedding_permission: EmbeddingPermission,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ManifestEntry {
    pub font: FontDescriptor,
    pub path: String,
    pub license: LicenseMetadata,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema_version: u32,
    pub resources: Vec<ManifestEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    InvalidManifest(String),
    MissingResource(String),
    MissingLicense(String),
    Io(String),
    SizeLimit,
    DigestMismatch,
    LicenseDigestMismatch,
    UnsupportedFont(String),
    InvalidFont(String),
    MetadataMismatch(&'static str),
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for Error {}
pub type Result<T> = std::result::Result<T, Error>;
fn invalid(message: &str) -> Error {
    Error::InvalidFont(message.into())
}
pub fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn valid_hash(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn check_entry(entry: &ManifestEntry) -> Result<()> {
    check_entry_common(entry)?;
    if entry.font.format != "static-truetype" || entry.font.face_index != 0 {
        return Err(Error::UnsupportedFont(
            "rendering-v2 permits static TrueType face 0 only".into(),
        ));
    }
    Ok(())
}
fn check_entry_common(entry: &ManifestEntry) -> Result<()> {
    let f = &entry.font;
    if f.font_id.is_empty()
        || f.font_id.len() > 128
        || !f.font_id.as_bytes()[0].is_ascii_alphanumeric()
        || !f
            .font_id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_.-".contains(&b))
        || !valid_hash(&f.sha256)
        || f.byte_length == 0
        || f.byte_length > MAX_FONT_BYTES as u64
    {
        return Err(Error::InvalidManifest(
            "invalid font identity, digest or byte length".into(),
        ));
    }
    if !(16..=16384).contains(&f.units_per_em)
        || !(2..=65536).contains(&f.glyph_count)
        || f.postscript_name.is_empty()
        || f.postscript_name.len() > 256
    {
        return Err(Error::InvalidManifest(
            "invalid declared font metrics/name".into(),
        ));
    }
    check_license_metadata(&entry.license)
}
fn check_license_metadata(l: &LicenseMetadata) -> Result<()> {
    if [&l.identifier, &l.copyright, &l.source]
        .iter()
        .any(|s| s.trim().is_empty() || s.len() > 8192)
        || !valid_hash(&l.text_sha256)
    {
        return Err(Error::InvalidManifest(
            "license provenance/digest required".into(),
        ));
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub struct FontResource {
    descriptor: FontDescriptor,
    license: LicenseMetadata,
    bytes: Arc<[u8]>,
    license_text: Arc<[u8]>,
    tables: BTreeMap<[u8; 4], Range<usize>>,
    embedding_flags: Option<u16>,
    cmap: Arc<std::sync::OnceLock<Result<mapping::Cmap>>>,
}
impl FontResource {
    /// Copies and verifies supplied bytes; later caller/file mutations cannot change this resource.
    pub fn from_bytes(
        entry: &ManifestEntry,
        font_bytes: &[u8],
        license_text: &[u8],
    ) -> Result<Self> {
        check_entry(entry)?;
        if font_bytes.len() > MAX_FONT_BYTES || license_text.len() > MAX_LICENSE_BYTES {
            return Err(Error::SizeLimit);
        }
        if font_bytes.len() as u64 != entry.font.byte_length {
            return Err(Error::MetadataMismatch("byte_length"));
        }
        if sha256(font_bytes) != entry.font.sha256 {
            return Err(Error::DigestMismatch);
        }
        if license_text.is_empty() || sha256(license_text) != entry.license.text_sha256 {
            return Err(Error::LicenseDigestMismatch);
        }
        let parsed = parse(font_bytes)?;
        if parsed.units != entry.font.units_per_em {
            return Err(Error::MetadataMismatch("units_per_em"));
        }
        if parsed.glyphs != entry.font.glyph_count {
            return Err(Error::MetadataMismatch("glyph_count"));
        }
        if !parsed
            .names
            .iter()
            .any(|name| name == &entry.font.postscript_name)
        {
            return Err(Error::MetadataMismatch("postscript_name"));
        }
        Ok(Self {
            descriptor: entry.font.clone(),
            license: entry.license.clone(),
            bytes: Arc::from(font_bytes),
            license_text: Arc::from(license_text),
            tables: parsed.tables,
            embedding_flags: parsed.embedding_flags,
            cmap: Arc::new(std::sync::OnceLock::new()),
        })
    }
    pub fn descriptor(&self) -> &FontDescriptor {
        &self.descriptor
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn shared_bytes(&self) -> Arc<[u8]> {
        Arc::clone(&self.bytes)
    }
    pub fn license(&self) -> &LicenseMetadata {
        &self.license
    }
    pub fn license_text(&self) -> &[u8] {
        &self.license_text
    }
    /// Raw OS/2 fsType, if supplied; consumers must enforce their embedding policy.
    pub fn embedding_flags(&self) -> Option<u16> {
        self.embedding_flags
    }
    pub fn table(&self, tag: &[u8; 4]) -> Option<&[u8]> {
        self.tables.get(tag).map(|range| &self.bytes[range.clone()])
    }
}

#[derive(Debug)]
pub struct FontCollection {
    resources: BTreeMap<String, FontResource>,
}
impl FontCollection {
    pub fn load(root: &Path, manifest: &Manifest) -> Result<Self> {
        if manifest.schema_version != 1 || manifest.resources.len() > 256 {
            return Err(Error::InvalidManifest(
                "unsupported manifest version/resource count".into(),
            ));
        }
        let mut entries = BTreeMap::new();
        let mut total = 0u64;
        for entry in &manifest.resources {
            check_entry(entry)?;
            total = total
                .checked_add(entry.font.byte_length)
                .ok_or(Error::SizeLimit)?;
            if total > MAX_COLLECTION_BYTES {
                return Err(Error::SizeLimit);
            }
            if entries.insert(entry.font.font_id.clone(), entry).is_some() {
                return Err(Error::InvalidManifest("duplicate font_id".into()));
            }
        }
        let mut resources = BTreeMap::new();
        for (id, entry) in entries {
            let bytes = read_resource(root, &entry.path, MAX_FONT_BYTES, false)?;
            let license = read_resource(root, &entry.license.text_path, MAX_LICENSE_BYTES, true)?;
            resources.insert(id, FontResource::from_bytes(entry, &bytes, &license)?);
        }
        Ok(Self { resources })
    }
    /// Exact ID lookup only. Missing fonts are errors; never choose a platform fallback.
    pub fn get(&self, id: &str) -> Result<&FontResource> {
        self.resources
            .get(id)
            .ok_or_else(|| Error::MissingResource(id.into()))
    }
    pub fn ids(&self) -> impl Iterator<Item = &str> {
        self.resources.keys().map(String::as_str)
    }
}
fn read_resource(root: &Path, relative: &str, limit: usize, license: bool) -> Result<Vec<u8>> {
    let path = Path::new(relative);
    if relative.is_empty()
        || relative.len() > 4096
        || relative.contains(['\\', ':', '\0'])
        || relative.ends_with('/')
        || relative
            .split('/')
            .any(|p| p.is_empty() || p == "." || p == "..")
        || path
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err(Error::InvalidManifest(
            "resource paths must be normalized relative paths".into(),
        ));
    }
    let root = root.canonicalize().map_err(|e| Error::Io(e.to_string()))?;
    let canonical = root.join(path).canonicalize().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            if license {
                Error::MissingLicense(relative.into())
            } else {
                Error::MissingResource(relative.into())
            }
        } else {
            Error::Io(e.to_string())
        }
    })?;
    if !canonical.starts_with(&root) {
        return Err(Error::InvalidManifest(
            "resource symlink escapes root".into(),
        ));
    }
    let file = File::open(canonical).map_err(|e| Error::Io(e.to_string()))?;
    if file.metadata().map_err(|e| Error::Io(e.to_string()))?.len() > limit as u64 {
        return Err(Error::SizeLimit);
    }
    let mut bytes = Vec::new();
    file.take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| Error::Io(e.to_string()))?;
    if bytes.len() > limit {
        return Err(Error::SizeLimit);
    }
    Ok(bytes)
}
fn u16_at(bytes: &[u8], at: usize) -> Result<u16> {
    let data = bytes
        .get(
            at..at
                .checked_add(2)
                .ok_or_else(|| invalid("offset overflow"))?,
        )
        .ok_or_else(|| invalid("truncated u16"))?;
    Ok(u16::from_be_bytes([data[0], data[1]]))
}
fn u32_at(bytes: &[u8], at: usize) -> Result<u32> {
    let data = bytes
        .get(
            at..at
                .checked_add(4)
                .ok_or_else(|| invalid("offset overflow"))?,
        )
        .ok_or_else(|| invalid("truncated u32"))?;
    Ok(u32::from_be_bytes([data[0], data[1], data[2], data[3]]))
}
/// Bounded sfnt metadata inspection for the rendering-core validation adapter.
/// This does not verify an expected digest or license; use FontResource for that.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontMetadata {
    pub units_per_em: u32,
    pub glyph_count: u32,
    pub postscript_names: Vec<String>,
    pub embedding_flags: Option<u16>,
}
pub fn inspect_static_truetype(bytes: &[u8]) -> Result<FontMetadata> {
    if bytes.len() > MAX_FONT_BYTES {
        return Err(Error::SizeLimit);
    }
    let parsed = parse(bytes)?;
    Ok(FontMetadata {
        units_per_em: parsed.units,
        glyph_count: parsed.glyphs,
        postscript_names: parsed.names,
        embedding_flags: parsed.embedding_flags,
    })
}
struct Parsed {
    tables: BTreeMap<[u8; 4], Range<usize>>,
    units: u32,
    glyphs: u32,
    names: Vec<String>,
    embedding_flags: Option<u16>,
}
fn parse(bytes: &[u8]) -> Result<Parsed> {
    if bytes.get(..4) != Some(&[0, 1, 0, 0]) {
        return Err(Error::UnsupportedFont(
            "only single-face TrueType sfnt version 1.0 accepted; no TTC/CFF/WOFF".into(),
        ));
    }
    let count = u16_at(bytes, 4)? as usize;
    if count == 0 || count > 4096 {
        return Err(invalid("invalid table count"));
    }
    let directory_end = 12 + count * 16;
    if directory_end > bytes.len() {
        return Err(invalid("truncated table directory"));
    }
    let mut tables = BTreeMap::new();
    let mut regions = Vec::new();
    for index in 0..count {
        let at = 12 + index * 16;
        let tag: [u8; 4] = bytes[at..at + 4].try_into().unwrap();
        let start = u32_at(bytes, at + 8)? as usize;
        let length = u32_at(bytes, at + 12)? as usize;
        let end = start
            .checked_add(length)
            .ok_or_else(|| invalid("table range overflow"))?;
        if !start.is_multiple_of(4) || start < directory_end || end > bytes.len() {
            return Err(invalid("table outside font or misaligned"));
        }
        if tables.insert(tag, start..end).is_some() {
            return Err(invalid("duplicate table tag"));
        }
        if length > 0 {
            regions.push(start..end);
        }
    }
    regions.sort_by_key(|r| r.start);
    if regions.windows(2).any(|pair| pair[0].end > pair[1].start) {
        return Err(invalid("overlapping font tables"));
    }
    if [b"fvar", b"gvar", b"CFF ", b"CFF2"]
        .iter()
        .any(|tag| tables.contains_key(*tag))
    {
        return Err(Error::UnsupportedFont(
            "variable/CFF outlines outside static-truetype profile".into(),
        ));
    }
    let table = |tag: &[u8; 4]| -> Result<&[u8]> {
        tables
            .get(tag)
            .map(|r| &bytes[r.clone()])
            .ok_or_else(|| invalid("required TrueType table missing"))
    };
    let head = table(b"head")?;
    let maxp = table(b"maxp")?;
    let loca = table(b"loca")?;
    let glyf = table(b"glyf")?;
    if head.len() < 54 || u32_at(head, 12)? != 0x5f0f3cf5 {
        return Err(invalid("invalid head table"));
    }
    let units = u16_at(head, 18)? as u32;
    if !(16..=16384).contains(&units) {
        return Err(invalid("invalid units-per-em"));
    }
    if maxp.len() < 32 || u32_at(maxp, 0)? != 0x00010000 {
        return Err(invalid("invalid TrueType maxp table"));
    }
    let glyphs = u16_at(maxp, 4)? as u32;
    if glyphs < 2 {
        return Err(invalid("at least two glyph slots required"));
    }
    let stride = match u16_at(head, 50)? {
        0 => 2,
        1 => 4,
        _ => return Err(invalid("invalid loca format")),
    };
    if loca.len() < (glyphs as usize + 1) * stride {
        return Err(invalid("truncated loca offsets"));
    }
    let mut previous = 0usize;
    for index in 0..=glyphs as usize {
        let offset = if stride == 2 {
            u16_at(loca, index * stride)? as usize * 2
        } else {
            u32_at(loca, index * stride)? as usize
        };
        if offset < previous
            || offset > glyf.len()
            || (index > 0 && offset > previous && offset - previous < 10)
        {
            return Err(invalid("invalid glyph offset/header extent"));
        }
        previous = offset;
    }
    let hhea = table(b"hhea")?;
    let hmtx = table(b"hmtx")?;
    if hhea.len() < 36 {
        return Err(invalid("truncated hhea table"));
    }
    let metrics = u16_at(hhea, 34)? as usize;
    if metrics == 0
        || metrics > glyphs as usize
        || hmtx.len() < metrics * 4 + (glyphs as usize - metrics) * 2
    {
        return Err(invalid("invalid horizontal metrics bounds"));
    }
    composite::validate(head, loca, glyf, glyphs as usize)?;
    let names = parse_names(table(b"name")?)?;
    let embedding_flags = match tables.get(b"OS/2") {
        Some(r) => Some(u16_at(&bytes[r.clone()], 8)?),
        None => None,
    };
    Ok(Parsed {
        tables,
        units,
        glyphs,
        names,
        embedding_flags,
    })
}
fn parse_names(bytes: &[u8]) -> Result<Vec<String>> {
    let format = u16_at(bytes, 0)?;
    if format > 1 {
        return Err(invalid("unsupported name table format"));
    }
    let count = u16_at(bytes, 2)? as usize;
    let storage = u16_at(bytes, 4)? as usize;
    if 6 + count * 12 > bytes.len() || storage < 6 + count * 12 || storage > bytes.len() {
        return Err(invalid("invalid name table bounds"));
    }
    let mut names = Vec::new();
    for index in 0..count {
        let at = 6 + index * 12;
        let platform = u16_at(bytes, at)?;
        let id = u16_at(bytes, at + 6)?;
        let length = u16_at(bytes, at + 8)? as usize;
        let start = storage + u16_at(bytes, at + 10)? as usize;
        let data = bytes
            .get(start..start + length)
            .ok_or_else(|| invalid("name string outside table"))?;
        if id != 6 {
            continue;
        }
        let name = if platform == 0 || platform == 3 {
            if data.len() % 2 != 0 {
                return Err(invalid("odd UTF-16 name length"));
            }
            let words: Vec<u16> = data
                .as_chunks::<2>()
                .0
                .iter()
                .map(|b| u16::from_be_bytes([b[0], b[1]]))
                .collect();
            String::from_utf16(&words).map_err(|_| invalid("invalid UTF-16 PostScript name"))?
        } else if platform == 1 && data.is_ascii() {
            String::from_utf8(data.to_vec()).unwrap()
        } else {
            continue;
        };
        if !name.is_empty() && name.len() <= 256 {
            names.push(name);
        }
    }
    if names.is_empty() {
        return Err(invalid("PostScript name missing"));
    }
    Ok(names)
}

mod mapping;
pub use mapping::HorizontalMetrics;

mod composite;
pub use composite::MAX_COMPOSITE_DEPTH;

mod outline;
pub use outline::{OutlinePoint, SimpleOutline};

mod expansion;
pub use expansion::{
    CompositeDeviceGrid, Coordinate, DeviceExpandedOutline, ExactPoint, ExpandedOutline,
    GlyphInstance, GridTieRule,
};

mod path;
pub use path::{PathCommand, QuadraticPath};

pub mod eexec;
pub mod pfb;
pub mod required_tfm;
pub mod tfm;
pub mod tfm_run;
pub mod type1_matrix;
pub mod type1_outline;
pub mod type1_records;

pub mod encoding;

pub mod vf;

pub mod vf_graph;

pub mod cff;

/// Identity-preserving adapter to the original sibling font engine.
pub mod engine_adapter;

/// Explicit project-scoped font registry using the rooted file layer.
pub mod registry;

pub mod math_adapter;

pub mod math_variants;

pub mod math_fit;

pub mod math_kern;

pub mod math_device;

pub mod math_cache;

pub mod enc_file;
