//! Disk discovery imports into authoritative private ledgers; rooted export
//! checks both source identity and expected disk content before writing.
use crate::Controller;
use flashtex_edit_ledger::{Document, Store};
use flashtex_project_files::{
    sha256_from_hex, sha256_hex, sha256_to_hex, Expected, Overlay, ProjectGraph, ProjectPath,
    ProjectRoot, SaveReceipt, DEFAULT_READ_LIMIT,
};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiskState {
    MatchesSource {
        sha256: String,
    },
    DiffersFromSource {
        disk_sha256: String,
        source_sha256: String,
    },
    Missing,
    Unavailable {
        reason: String,
    },
}
pub struct FileProject {
    root: PathBuf,
    capability: ProjectRoot,
    ledger_root: PathBuf,
    project_id: String,
    diagnostics: Vec<String>,
}
impl FileProject {
    /// Existing ledgers take precedence over disk. Discovery uses their source as
    /// overlays so an externally deleted entry can still reopen without data loss.
    /// `private_root` must already be an application-owned directory.
    pub fn open(
        root: &Path,
        private_root: &Path,
        project_id: &str,
        entry: &str,
    ) -> Result<(Self, Controller), String> {
        Self::open_with_bibliography(root, private_root, project_id, entry, &[])
    }
    /// Explicit bibliography paths are imported through the same rooted capability
    /// and retained private ledgers as ordinary source documents.
    pub fn open_with_bibliography(
        root: &Path,
        private_root: &Path,
        project_id: &str,
        entry: &str,
        bibliography_paths: &[String],
    ) -> Result<(Self, Controller), String> {
        let mut declared = std::collections::BTreeSet::new();
        for path in bibliography_paths {
            let normalized = ProjectPath::normalize(path).map_err(|e| e.to_string())?;
            if normalized.as_str() != path || path == entry || !declared.insert(path) {
                return Err("invalid, duplicate or entry bibliography path".into());
            }
        }
        let root = root.canonicalize().map_err(|e| e.to_string())?;
        if !root.is_dir() {
            return Err("project root is not a directory".into());
        }
        let private_root = private_root.canonicalize().map_err(|e| e.to_string())?;
        if !private_root.is_dir() {
            return Err("private ledger root is not a directory".into());
        }
        let capability = ProjectRoot::open(&root).map_err(|e| e.to_string())?;
        let entry = ProjectPath::normalize(entry).map_err(|e| e.to_string())?;
        let root_text = root
            .to_str()
            .ok_or("project root must be UTF-8 for stable binding")?;
        let binding =
            sha256_hex(format!("{}:{root_text}:{project_id}", root_text.len()).as_bytes());
        let ledger_root = private_root.join(format!("project-{binding}"));
        if fs::symlink_metadata(&ledger_root).is_ok_and(|meta| meta.file_type().is_symlink()) {
            return Err("private ledger binding cannot be a symlink".into());
        }
        fs::create_dir_all(&ledger_root).map_err(|e| e.to_string())?;
        let mut stores = BTreeMap::new();
        let mut overlay = Overlay::new();
        for item in fs::read_dir(&ledger_root).map_err(|e| e.to_string())? {
            let item = item.map_err(|e| e.to_string())?;
            let name = item
                .file_name()
                .into_string()
                .map_err(|_| "invalid ledger name")?;
            if name.len() != 64 || !name.bytes().all(|b| b.is_ascii_hexdigit()) {
                continue;
            }
            if !item.file_type().map_err(|e| e.to_string())?.is_dir() {
                return Err("ledger slot is not a private directory".into());
            }
            let store = Store::open(item.path()).map_err(|e| e.to_string())?;
            if let Some(document) = store.document().map_err(|e| e.to_string())? {
                let path = ProjectPath::normalize(&document.path).map_err(|e| e.to_string())?;
                if document.project_id != project_id
                    || path.as_str() != document.path
                    || sha256_hex(document.path.as_bytes()) != name
                {
                    return Err("ledger binding disagrees with document identity".into());
                }
                overlay.insert(path, document.text.clone());
                stores.insert(document.path.clone(), store);
                if stores.len() > 256 {
                    return Err("project exceeds 256 source stores".into());
                }
            }
        }
        if overlay.get(&entry).is_none() {
            check_entry(&root, &entry)?;
        }
        let graph =
            ProjectGraph::discover_with(&root, &entry, &overlay).map_err(|e| e.to_string())?;
        for file in graph.files() {
            let Some(text) = &file.text else {
                continue;
            };
            if stores.contains_key(file.path.as_str()) {
                continue;
            }
            if stores.len() >= 256 {
                return Err("project exceeds 256 source stores".into());
            }
            let mut store =
                Store::open(ledger_root.join(sha256_hex(file.path.as_str().as_bytes())))
                    .map_err(|e| e.to_string())?;
            store
                .initialize(
                    Document::new(
                        project_id.into(),
                        file.path.as_str().into(),
                        1,
                        text.clone(),
                    )
                    .map_err(|e| e.to_string())?,
                )
                .map_err(|e| e.to_string())?;
            stores.insert(file.path.as_str().to_owned(), store);
        }
        for path in bibliography_paths {
            if stores.contains_key(path) {
                continue;
            }
            if stores.len() >= 256 {
                return Err("project exceeds 256 source stores".into());
            }
            let normalized = ProjectPath::normalize(path).map_err(|e| e.to_string())?;
            let file = capability
                .read(&normalized, flashtex_edit_ledger::MAX_DOCUMENT_BYTES as u64)
                .map_err(|e| e.to_string())?
                .ok_or("bibliography source missing on disk")?;
            let text =
                String::from_utf8(file.bytes).map_err(|_| "bibliography source must be UTF-8")?;
            let slot = ledger_root.join(sha256_hex(path.as_bytes()));
            if fs::symlink_metadata(&slot).is_ok_and(|meta| !meta.is_dir()) {
                return Err("ledger slot is not a private directory".into());
            }
            let mut store = Store::open(slot).map_err(|e| e.to_string())?;
            store
                .initialize(
                    Document::new(project_id.into(), path.clone(), 1, text)
                        .map_err(|e| e.to_string())?,
                )
                .map_err(|e| e.to_string())?;
            stores.insert(path.clone(), store);
        }
        let diagnostics = graph
            .diagnostics()
            .iter()
            .map(|diagnostic| diagnostic.message.clone())
            .collect();
        let controller = Controller::open_with_bibliography(
            project_id.into(),
            entry.as_str().into(),
            stores.into_values().collect(),
            bibliography_paths,
        )?;
        Ok((
            Self {
                root,
                capability,
                ledger_root,
                project_id: project_id.into(),
                diagnostics,
            },
            controller,
        ))
    }
    pub fn root(&self) -> &Path {
        &self.root
    }
    pub fn diagnostics(&self) -> &[String] {
        &self.diagnostics
    }
    /// Bounded rooted read; never changes authoritative source or a disk baseline.
    pub fn inspect(&self, controller: &Controller, path: &str) -> Result<DiskState, String> {
        let source = controller.document(path)?;
        if source.project_id != self.project_id {
            return Err("controller belongs to another project".into());
        }
        let path = ProjectPath::normalize(path).map_err(|e| e.to_string())?;
        match self.capability.read(&path, DEFAULT_READ_LIMIT) {
            Ok(Some(file)) => {
                let hash = sha256_to_hex(&file.sha256);
                Ok(if hash == source.source_sha256 {
                    DiskState::MatchesSource { sha256: hash }
                } else {
                    DiskState::DiffersFromSource {
                        disk_sha256: hash,
                        source_sha256: source.source_sha256.clone(),
                    }
                })
            }
            Ok(None) => Ok(DiskState::Missing),
            Err(error) => Ok(DiskState::Unavailable {
                reason: error.to_string(),
            }),
        }
    }
    /// Explicitly open another existing disk file, preferring a retained ledger.
    /// Import creates private source only; it never creates or edits a disk file.
    pub fn open_document(
        &self,
        controller: &mut Controller,
        expected: &flashtex_project_index::VersionSnapshot,
        path: &str,
    ) -> Result<crate::EditOutcome, String> {
        self.open_document_with_kind(
            controller,
            expected,
            path,
            flashtex_project_index::DocumentKind::Latex,
        )
    }
    pub fn open_document_with_kind(
        &self,
        controller: &mut Controller,
        expected: &flashtex_project_index::VersionSnapshot,
        path: &str,
        kind: flashtex_project_index::DocumentKind,
    ) -> Result<crate::EditOutcome, String> {
        if expected != &controller.index().snapshot() || expected.project_id != self.project_id {
            return Err("project membership snapshot is stale".into());
        }
        let normalized = ProjectPath::normalize(path).map_err(|e| e.to_string())?;
        if normalized.as_str() != path {
            return Err("document path must be normalized".into());
        }
        if controller.document(path).is_ok() {
            return Err("document already attached".into());
        }
        let slot = self.ledger_root.join(sha256_hex(path.as_bytes()));
        if fs::symlink_metadata(&slot).is_ok_and(|meta| !meta.is_dir()) {
            return Err("ledger slot is not a private directory".into());
        }
        let mut store = Store::open(slot).map_err(|e| e.to_string())?;
        match store.document().map_err(|e| e.to_string())? {
            Some(doc) if doc.project_id != self.project_id || doc.path != path => {
                return Err("retained ledger identity differs from requested document".into());
            }
            Some(_) => {}
            None => {
                let file = self
                    .capability
                    .read(&normalized, flashtex_edit_ledger::MAX_DOCUMENT_BYTES as u64)
                    .map_err(|e| e.to_string())?
                    .ok_or("document is missing on disk")?;
                let text = String::from_utf8(file.bytes).map_err(|_| "source must be UTF-8")?;
                store
                    .initialize(
                        Document::new(self.project_id.clone(), path.into(), 1, text)
                            .map_err(|e| e.to_string())?,
                    )
                    .map_err(|e| e.to_string())?;
            }
        }
        controller.attach_document_with_kind(expected, store, kind)
    }

    /// Explicitly accept a reviewed disk snapshot into durable source. The old
    /// source remains in ledger undo history. No disk writes or automatic reload.
    pub fn reload_explicitly(
        &self,
        controller: &mut Controller,
        path: &str,
        expected_revision: u64,
        expected_source_sha256: &str,
        expected_disk_sha256: &str,
    ) -> Result<crate::EditOutcome, String> {
        let source = controller.document(path)?;
        if source.project_id != self.project_id
            || source.revision != expected_revision
            || source.source_sha256 != expected_source_sha256
        {
            return Err("reload source identity is stale or belongs to another project".into());
        }
        let expected =
            sha256_from_hex(expected_disk_sha256).ok_or("invalid expected disk SHA-256")?;
        let normalized = ProjectPath::normalize(path).map_err(|e| e.to_string())?;
        let _lock = self.capability.lock().map_err(|e| e.to_string())?;
        let file = self
            .capability
            .read(&normalized, flashtex_edit_ledger::MAX_DOCUMENT_BYTES as u64)
            .map_err(|e| e.to_string())?
            .ok_or("reload target is missing")?;
        if file.sha256 != expected {
            return Err("disk changed since reload was reviewed".into());
        }
        let text = String::from_utf8(file.bytes).map_err(|_| "reload source must be UTF-8")?;
        controller.replace_document(path, expected_revision, expected_source_sha256, text)
    }

    /// None means the target must not exist. No force-overwrite option.
    /// A post-rename error may mean bytes changed: inspect before retrying.
    pub fn export(
        &self,
        controller: &Controller,
        path: &str,
        expected_revision: u64,
        expected_source_sha256: &str,
        expected_disk_sha256: Option<&str>,
    ) -> Result<SaveReceipt, String> {
        let source = controller.document(path)?;
        if source.project_id != self.project_id
            || source.revision != expected_revision
            || source.source_sha256 != expected_source_sha256
        {
            return Err("export source identity is stale or belongs to another project".into());
        }
        let path = ProjectPath::normalize(path).map_err(|e| e.to_string())?;
        let expected = match expected_disk_sha256 {
            Some(hash) => {
                Expected::Hash(sha256_from_hex(hash).ok_or("invalid expected disk SHA-256")?)
            }
            None => Expected::NewFile,
        };
        self.capability
            .save(&path, source.text.as_bytes(), expected, false)
            .map_err(|e| format!("export failed; inspect disk before retrying: {e}"))
    }
}

// Consumer preflight for the shared graph's unchecked entry path. This is not a
// race-proof directory capability; shared rooted IO is tracked in GH18.
fn check_entry(root: &Path, path: &ProjectPath) -> Result<(), String> {
    match path.to_os_path(root).canonicalize() {
        Ok(actual) if actual.starts_with(root) => Ok(()),
        Ok(_) => Err("entry resolves outside bound project root".into()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.to_string()),
    }
}
