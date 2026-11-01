//! Explicit required metric assets. A failed load never authorizes OTF substitution.
use crate::{sha256, tfm::Tfm};
use flashtex_project_files::{ProjectPath, ProjectRoot};
use std::collections::BTreeMap;
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MetricAsset {
    pub path: String,
    pub sha256: String,
    pub license_path: String,
    pub license_sha256: String,
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema_version: u32,
    pub metrics: Vec<MetricAsset>,
}
#[derive(Debug)]
pub enum Error {
    Manifest,
    Path,
    Missing(String),
    Digest(String),
    Read(String),
    Parse(crate::Error),
}
pub struct RequiredMetrics {
    entries: BTreeMap<String, (MetricAsset, Tfm)>,
}
impl RequiredMetrics {
    /// All-or-error load; at most16 metric files, 128KiB each and16KiB/license.
    pub fn load(root: &ProjectRoot, manifest: &Manifest) -> Result<Self, Error> {
        if manifest.schema_version != 1
            || manifest.metrics.is_empty()
            || manifest.metrics.len() > 16
        {
            return Err(Error::Manifest);
        }
        let mut entries = BTreeMap::new();
        for asset in &manifest.metrics {
            if entries.contains_key(&asset.path) {
                return Err(Error::Manifest);
            }
            let bytes = read(root, &asset.path, &asset.sha256, 128 * 1024)?;
            read(root, &asset.license_path, &asset.license_sha256, 16 * 1024)?;
            let tfm = Tfm::parse(&bytes).map_err(Error::Parse)?;
            entries.insert(asset.path.clone(), (asset.clone(), tfm));
        }
        Ok(Self { entries })
    }
    pub fn get(&self, path: &str) -> Result<(&MetricAsset, &Tfm), Error> {
        self.entries
            .get(path)
            .map(|(a, t)| (a, t))
            .ok_or_else(|| Error::Missing(path.into()))
    }
}
fn read(root: &ProjectRoot, path: &str, digest: &str, cap: u64) -> Result<Vec<u8>, Error> {
    if path.len() > 4096
        || digest.len() != 64
        || !digest
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    {
        return Err(Error::Manifest);
    }
    let p = ProjectPath::normalize(path).map_err(|_| Error::Path)?;
    if p.as_str() != path {
        return Err(Error::Path);
    }
    let f = root
        .read(&p, cap)
        .map_err(|e| Error::Read(e.to_string()))?
        .ok_or_else(|| Error::Missing(path.into()))?;
    if sha256(&f.bytes) != digest {
        return Err(Error::Digest(path.into()));
    }
    Ok(f.bytes)
}
