//! Atomically published checkpoint index with post-commit retention cleanup.
//! The old index and all its files survive until the new index is fsynced.
use super::{checksum, Checkpoint, ExportAuthorization, StoreIdentity, MAX_CHECKPOINT_BYTES};
use crate::{Error, Result, Store};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::Path,
};

pub const MAX_ARCHIVE_BYTES: u64 = 512 * 1024 * 1024;
const MAX_INDEX_BYTES: u64 = 128 * 1024;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RotationPolicy {
    pub acknowledge_checkpoint_deletion: bool,
    pub keep_latest: usize,
    pub max_total_bytes: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CheckpointInfo {
    pub generation: u64,
    pub file_name: String,
    pub checkpoint_digest: String,
    pub source_revision: u64,
    pub bytes: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RotationReport {
    pub created: CheckpointInfo,
    pub retained: Vec<CheckpointInfo>,
    pub removed_files: Vec<String>,
}
/// Ledger-local metadata for a native recovery screen; contains no source text.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointStatus {
    pub identity: StoreIdentity,
    pub current_revision: u64,
    pub current_source_sha256: String,
    pub checkpoints: Vec<CheckpointInfo>,
    pub retained_bytes: u64,
    pub unindexed_generations: Vec<u64>,
    pub interrupted_write: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Index {
    format_version: u8,
    identity: StoreIdentity,
    checkpoints: Vec<CheckpointInfo>,
    integrity_sha256: String,
}
impl Index {
    fn checksum(&self) -> Result<String> {
        checksum(&(self.format_version, &self.identity, &self.checkpoints))
    }
}
fn file_name(generation: u64, digest: &str) -> String {
    format!("{generation:020}-{digest}.json")
}
fn checked_read(path: &Path, limit: u64) -> Result<Vec<u8>> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.file_type().is_file() || metadata.len() > limit {
        return Err(Error::new(
            "invalid_archive_file",
            "expected bounded regular checkpoint file",
        ));
    }
    let mut bytes = Vec::new();
    File::open(path)?.take(limit + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        return Err(Error::new(
            "invalid_archive_file",
            "archive file grew past size limit",
        ));
    }
    Ok(bytes)
}
fn parse_name(name: &str) -> Option<(u64, &str)> {
    let (generation, digest) = name.strip_suffix(".json")?.split_once('-')?;
    if generation.len() != 20
        || digest.len() != 64
        || !generation.bytes().all(|b| b.is_ascii_digit())
        || !digest.bytes().all(|b| b.is_ascii_hexdigit())
    {
        return None;
    }
    let generation = generation.parse().ok()?;
    if generation == 0 {
        return None;
    }
    Some((generation, digest))
}
fn checkpoint_info(root: &Path, name: &str, identity: &StoreIdentity) -> Result<CheckpointInfo> {
    let (generation, digest) = parse_name(name)
        .ok_or_else(|| Error::new("invalid_archive_file", "invalid checkpoint filename"))?;
    let bytes = checked_read(&root.join(name), MAX_CHECKPOINT_BYTES as u64)?;
    let checkpoint = Checkpoint::decode(&bytes)?;
    if &checkpoint.identity != identity || checkpoint.integrity_sha256 != digest {
        return Err(Error::new(
            "checkpoint_identity_conflict",
            "archive checkpoint identity/digest differs",
        ));
    }
    Ok(CheckpointInfo {
        generation,
        file_name: name.into(),
        checkpoint_digest: digest.into(),
        source_revision: checkpoint.document().revision,
        bytes: bytes.len() as u64,
    })
}
fn choose(
    mut entries: Vec<CheckpointInfo>,
    policy: &RotationPolicy,
) -> Result<Vec<CheckpointInfo>> {
    entries.sort_by_key(|e| e.generation);
    while entries.len() > policy.keep_latest
        || entries.iter().map(|e| e.bytes).sum::<u64>() > policy.max_total_bytes
    {
        if entries.len() <= 1 {
            return Err(Error::new(
                "checkpoint_budget",
                "latest recoverable checkpoint exceeds retention budget",
            ));
        }
        entries.remove(0);
    }
    Ok(entries)
}
impl Store {
    fn archive_identity(&self) -> Result<StoreIdentity> {
        self.ready()?;
        let state = self
            .state
            .as_ref()
            .ok_or_else(|| Error::new("document_missing", "initialize source first"))?;
        if state.store_id.is_empty() {
            return Err(Error::new(
                "checkpoint_identity_missing",
                "export a checkpoint to establish legacy store identity",
            ));
        }
        Ok(StoreIdentity::from_state(state))
    }
    fn read_checkpoint_index(&self) -> Result<Option<Index>> {
        let root = self.root.join("checkpoints");
        match fs::symlink_metadata(&root) {
            Ok(m) if m.file_type().is_dir() => {}
            Ok(_) => {
                return Err(Error::new(
                    "invalid_archive_file",
                    "archive must be a regular directory",
                ))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.into()),
        }
        let path = root.join("index.json");
        match fs::symlink_metadata(&path) {
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        }
        let bytes = checked_read(&path, MAX_INDEX_BYTES)?;
        let index: Index = serde_json::from_slice(&bytes)
            .map_err(|e| Error::new("invalid_checkpoint_index", e.to_string()))?;
        if index.format_version != 1
            || index.identity != self.archive_identity()?
            || index.integrity_sha256 != index.checksum()?
            || index.checkpoints.len() > 32
        {
            return Err(Error::new(
                "invalid_checkpoint_index",
                "index format, identity, count or integrity mismatch",
            ));
        }
        let mut generations = BTreeSet::new();
        for info in &index.checkpoints {
            if info.file_name != file_name(info.generation, &info.checkpoint_digest)
                || !generations.insert(info.generation)
                || checkpoint_info(&root, &info.file_name, &index.identity)? != *info
            {
                return Err(Error::new(
                    "invalid_checkpoint_index",
                    "index does not match durable checkpoint files",
                ));
            }
        }
        Ok(Some(index))
    }
    pub fn list_checkpoints(&self) -> Result<Vec<CheckpointInfo>> {
        self.ready()?;
        Ok(self
            .read_checkpoint_index()?
            .map(|i| i.checkpoints)
            .unwrap_or_default())
    }
    pub fn checkpoint_status(&self) -> Result<CheckpointStatus> {
        let identity = self.archive_identity()?;
        let root = self.root.join("checkpoints");
        let present = match fs::symlink_metadata(&root) {
            Ok(m) if m.file_type().is_dir() => true,
            Ok(_) => {
                return Err(Error::new(
                    "invalid_archive_file",
                    "archive must be a regular directory",
                ))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => false,
            Err(e) => return Err(e.into()),
        };
        let checkpoints = self.list_checkpoints()?;
        let physical = if present {
            self.physical_checkpoints(&identity)?
        } else {
            Vec::new()
        };
        let unindexed_generations: Vec<_> = physical
            .iter()
            .filter(|info| !checkpoints.contains(info))
            .map(|info| info.generation)
            .collect();
        let mut interrupted_write = !unindexed_generations.is_empty();
        for name in [".checkpoint-pending", ".index-pending"] {
            match fs::symlink_metadata(root.join(name)) {
                Ok(m) if m.file_type().is_file() => interrupted_write = true,
                Ok(_) => {
                    return Err(Error::new(
                        "invalid_archive_file",
                        "pending archive path must be a regular file",
                    ))
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(e.into()),
            }
        }
        let document = &self.state.as_ref().unwrap().document;
        Ok(CheckpointStatus {
            identity,
            current_revision: document.revision,
            current_source_sha256: document.source_sha256.clone(),
            retained_bytes: checkpoints.iter().map(|info| info.bytes).sum(),
            checkpoints,
            unindexed_generations,
            interrupted_write,
        })
    }
    pub fn read_checkpoint(&self, generation: u64) -> Result<Checkpoint> {
        let info = self
            .list_checkpoints()?
            .into_iter()
            .find(|i| i.generation == generation)
            .ok_or_else(|| {
                Error::new(
                    "checkpoint_missing",
                    "generation is not in committed checkpoint index",
                )
            })?;
        Checkpoint::decode(&checked_read(
            &self.root.join("checkpoints").join(info.file_name),
            MAX_CHECKPOINT_BYTES as u64,
        )?)
    }
    fn publish_archive_file(&self, temporary: &str, final_name: &str, bytes: &[u8]) -> Result<()> {
        let root = self.root.join("checkpoints");
        let pending = root.join(temporary);
        match fs::symlink_metadata(&pending) {
            Ok(m) if m.file_type().is_file() => fs::remove_file(&pending)?,
            Ok(_) => {
                return Err(Error::new(
                    "invalid_archive_file",
                    "pending archive path is not a regular file",
                ))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
        let mut options = OpenOptions::new();
        options.create_new(true).write(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(&pending)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        #[cfg(test)]
        if temporary == ".checkpoint-pending" {
            self.inject("checkpoint_before_publish")?;
        }
        fs::rename(pending, root.join(final_name))?;
        crate::open_dir_for_sync(&root)?.sync_all()?;
        Ok(())
    }
    fn publish_index(
        &self,
        identity: StoreIdentity,
        checkpoints: Vec<CheckpointInfo>,
    ) -> Result<()> {
        let mut index = Index {
            format_version: 1,
            identity,
            checkpoints,
            integrity_sha256: String::new(),
        };
        index.integrity_sha256 = index.checksum()?;
        let bytes = serde_json::to_vec(&index)
            .map_err(|e| Error::new("invalid_checkpoint_index", e.to_string()))?;
        if bytes.len() as u64 > MAX_INDEX_BYTES {
            return Err(Error::new(
                "checkpoint_index_limit",
                "index metadata exceeds limit",
            ));
        }
        self.publish_archive_file(".index-pending", "index.json", &bytes)
    }
    fn physical_checkpoints(&self, identity: &StoreIdentity) -> Result<Vec<CheckpointInfo>> {
        let root = self.root.join("checkpoints");
        let mut infos = Vec::new();
        let mut seen = 0;
        for entry in fs::read_dir(&root)? {
            let entry = entry?;
            seen += 1;
            if seen > 68 {
                return Err(Error::new(
                    "checkpoint_archive_limit",
                    "too many archive entries; inspect interrupted rotation",
                ));
            }
            let name = entry
                .file_name()
                .into_string()
                .map_err(|_| Error::new("invalid_archive_file", "non-UTF8 archive filename"))?;
            if ["index.json", ".checkpoint-pending", ".index-pending"].contains(&name.as_str()) {
                continue;
            }
            infos.push(checkpoint_info(&root, &name, identity)?);
        }
        infos.sort_by_key(|i| i.generation);
        Ok(infos)
    }
    fn clean_unreferenced(
        &self,
        physical: &[CheckpointInfo],
        retained: &[CheckpointInfo],
    ) -> Result<Vec<String>> {
        if retained.is_empty() {
            return Ok(Vec::new());
        }
        let root = self.root.join("checkpoints");
        let keep: BTreeSet<_> = retained.iter().map(|i| &i.file_name).collect();
        let mut removed = Vec::new();
        for info in physical {
            if !keep.contains(&info.file_name) {
                fs::remove_file(root.join(&info.file_name))?;
                removed.push(info.file_name.clone());
            }
        }
        crate::open_dir_for_sync(&root)?.sync_all()?;
        Ok(removed)
    }
    /// Publish the new checkpoint and then its authoritative index atomically.
    /// Old files are pruned only after the new index is durable. Retry reconciles
    /// valid unindexed files from an interrupted first rotation before deleting.
    pub fn rotate_checkpoint(
        &mut self,
        authorization: ExportAuthorization,
        policy: RotationPolicy,
    ) -> Result<RotationReport> {
        if !policy.acknowledge_checkpoint_deletion {
            return Err(Error::new(
                "retention_not_acknowledged",
                "checkpoint rotation requires explicit retention acknowledgement",
            ));
        }
        if !(1..=32).contains(&policy.keep_latest)
            || policy.max_total_bytes == 0
            || policy.max_total_bytes > MAX_ARCHIVE_BYTES
        {
            return Err(Error::new(
                "invalid_rotation_policy",
                "retention is bounded to 1–32 checkpoints and 512 MiB",
            ));
        }
        let checkpoint = self.export_checkpoint(authorization)?;
        let bytes = checkpoint.encode()?;
        if bytes.len() as u64 > policy.max_total_bytes {
            return Err(Error::new(
                "checkpoint_budget",
                "new checkpoint exceeds requested byte budget",
            ));
        }
        let identity = checkpoint.identity.clone();
        let root = self.root.join("checkpoints");
        #[cfg_attr(not(unix), allow(unused_mut))]
        let mut builder = fs::DirBuilder::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        match builder.create(&root) {
            Ok(()) => crate::open_dir_for_sync(&self.root)?.sync_all()?,
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                if !fs::symlink_metadata(&root)?.file_type().is_dir() {
                    return Err(Error::new(
                        "invalid_archive_file",
                        "archive must be a regular directory",
                    ));
                }
            }
            Err(e) => return Err(e.into()),
        }
        let physical = self.physical_checkpoints(&identity)?;
        let mut retained = match self.read_checkpoint_index()? {
            Some(index) => index.checkpoints,
            None if !physical.is_empty() => {
                let recovered = choose(physical.clone(), &policy)?;
                self.publish_index(identity.clone(), recovered.clone())?;
                recovered
            }
            None => Vec::new(),
        };
        // Reconcile a completed-but-unacknowledged prior index before another
        // full checkpoint is written, keeping disk usage bounded after crashes.
        let normalized = choose(retained.clone(), &policy)?;
        if normalized != retained {
            self.publish_index(identity.clone(), normalized.clone())?;
            retained = normalized;
        }
        // A previous process may have renamed the index but died before its
        // directory sync. Establish that observed index durably before pruning
        // any file an older on-disk index could still reference.
        if !retained.is_empty() {
            // Write access is required for `sync_all` on Windows even though
            // this handle never writes (see `open_dir_for_sync` in lib.rs).
            OpenOptions::new()
                .write(true)
                .open(root.join("index.json"))?
                .sync_all()?;
            crate::open_dir_for_sync(&root)?.sync_all()?;
        }
        let mut removed_files = self.clean_unreferenced(&physical, &retained)?;
        let generation = physical
            .iter()
            .map(|i| i.generation)
            .max()
            .unwrap_or(0)
            .checked_add(1)
            .ok_or_else(|| {
                Error::new(
                    "checkpoint_generation_overflow",
                    "archive generation exhausted",
                )
            })?;
        let created = CheckpointInfo {
            generation,
            file_name: file_name(generation, &checkpoint.integrity_sha256),
            checkpoint_digest: checkpoint.integrity_sha256.clone(),
            source_revision: checkpoint.document().revision,
            bytes: bytes.len() as u64,
        };
        self.publish_archive_file(".checkpoint-pending", &created.file_name, &bytes)?;
        #[cfg(test)]
        self.inject("checkpoint_before_index")?;
        let mut candidates = retained.clone();
        candidates.push(created.clone());
        let selected = choose(candidates.clone(), &policy)?;
        self.publish_index(identity, selected.clone())?;
        #[cfg(test)]
        self.inject("checkpoint_after_index")?;
        removed_files.extend(self.clean_unreferenced(&candidates, &selected)?);
        Ok(RotationReport {
            created,
            retained: selected,
            removed_files,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::setup;
    fn policy() -> RotationPolicy {
        RotationPolicy {
            acknowledge_checkpoint_deletion: true,
            keep_latest: 1,
            max_total_bytes: MAX_ARCHIVE_BYTES,
        }
    }
    fn auth() -> ExportAuthorization {
        ExportAuthorization {
            acknowledge_private_source_export: true,
        }
    }
    #[test]
    fn rotation_keeps_bounded_latest_and_preserves_durable_ids() {
        let (_dir, mut store, edit) = setup();
        let first = store.rotate_checkpoint(auth(), policy()).unwrap();
        let receipt = store.apply(edit.clone()).unwrap();
        let second = store.rotate_checkpoint(auth(), policy()).unwrap();
        assert_eq!(second.retained.len(), 1);
        assert!(second.created.generation > first.created.generation);
        assert_eq!(second.removed_files.len(), 1);
        let cp = store.read_checkpoint(second.created.generation).unwrap();
        assert_eq!(cp.document().text, "a$x$z");
        assert_eq!(store.apply(edit).unwrap(), receipt);
    }
    #[test]
    fn failures_before_publication_or_index_leave_old_checkpoint_recoverable() {
        for point in ["checkpoint_before_publish", "checkpoint_before_index"] {
            let (dir, mut store, edit) = setup();
            let first = store.rotate_checkpoint(auth(), policy()).unwrap();
            store.apply(edit).unwrap();
            store.failpoint = Some(point);
            assert!(store.rotate_checkpoint(auth(), policy()).is_err());
            drop(store);
            let mut store = Store::open(dir.path()).unwrap();
            assert_eq!(
                store
                    .read_checkpoint(first.created.generation)
                    .unwrap()
                    .document()
                    .text,
                "aé😀z"
            );
            let recovered = store.rotate_checkpoint(auth(), policy()).unwrap();
            assert_eq!(recovered.retained.len(), 1);
            assert_eq!(
                store
                    .read_checkpoint(recovered.created.generation)
                    .unwrap()
                    .document()
                    .text,
                "a$x$z"
            );
        }
    }
    #[test]
    fn failure_after_index_keeps_new_checkpoint_and_retry_cleans_orphans() {
        let (dir, mut store, edit) = setup();
        store.rotate_checkpoint(auth(), policy()).unwrap();
        store.apply(edit).unwrap();
        store.failpoint = Some("checkpoint_after_index");
        assert!(store.rotate_checkpoint(auth(), policy()).is_err());
        drop(store);
        let mut store = Store::open(dir.path()).unwrap();
        let committed = store.list_checkpoints().unwrap();
        assert_eq!(committed.len(), 1);
        assert_eq!(
            store
                .read_checkpoint(committed[0].generation)
                .unwrap()
                .document()
                .text,
            "a$x$z"
        );
        let done = store.rotate_checkpoint(auth(), policy()).unwrap();
        assert_eq!(done.retained.len(), 1);
        assert_eq!(
            store
                .physical_checkpoints(&store.archive_identity().unwrap())
                .unwrap()
                .len(),
            1
        );
    }
    #[test]
    fn interrupted_first_index_is_reconciled_without_losing_only_backup() {
        let (dir, mut store, _) = setup();
        store.failpoint = Some("checkpoint_before_index");
        assert!(store.rotate_checkpoint(auth(), policy()).is_err());
        drop(store);
        let mut store = Store::open(dir.path()).unwrap();
        assert!(store.list_checkpoints().unwrap().is_empty());
        let status = store.checkpoint_status().unwrap();
        assert!(status.interrupted_write);
        assert_eq!(status.unindexed_generations, vec![1]);
        let done = store.rotate_checkpoint(auth(), policy()).unwrap();
        assert_eq!(done.created.generation, 2);
        assert_eq!(done.retained.len(), 1);
        assert!(!store.checkpoint_status().unwrap().interrupted_write);
    }
    #[cfg(unix)]
    #[test]
    fn archive_directory_symlink_is_rejected_without_external_writes() {
        let (dir, mut store, _) = setup();
        let outside = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink(outside.path(), dir.path().join("checkpoints")).unwrap();
        assert_eq!(
            store.rotate_checkpoint(auth(), policy()).unwrap_err().code,
            "invalid_archive_file"
        );
        assert!(store.list_checkpoints().is_err());
        assert!(store.checkpoint_status().is_err());
        assert_eq!(fs::read_dir(outside.path()).unwrap().count(), 0);
    }
    #[test]
    fn policy_denial_and_corrupt_index_never_prune_files() {
        let (_dir, mut store, _) = setup();
        let first = store.rotate_checkpoint(auth(), policy()).unwrap();
        let mut denied = policy();
        denied.acknowledge_checkpoint_deletion = false;
        assert_eq!(
            store.rotate_checkpoint(auth(), denied).unwrap_err().code,
            "retention_not_acknowledged"
        );
        fs::write(store.root.join("checkpoints/index.json"), b"{broken").unwrap();
        assert!(store.rotate_checkpoint(auth(), policy()).is_err());
        assert!(store
            .root
            .join("checkpoints")
            .join(first.created.file_name)
            .exists());
    }
    #[test]
    fn byte_budget_prunes_oldest_and_oversize_refusal_preserves_index() {
        let (_dir, mut store, _) = setup();
        let first = store.rotate_checkpoint(auth(), policy()).unwrap();
        let mut bounded = policy();
        bounded.keep_latest = 3;
        bounded.max_total_bytes = first.created.bytes * 2;
        store.rotate_checkpoint(auth(), bounded.clone()).unwrap();
        let third = store.rotate_checkpoint(auth(), bounded.clone()).unwrap();
        assert_eq!(third.retained.len(), 2);
        assert!(third
            .retained
            .iter()
            .all(|info| info.generation > first.created.generation));
        assert!(store.checkpoint_status().unwrap().retained_bytes <= bounded.max_total_bytes);
        bounded.max_total_bytes = 1;
        assert_eq!(
            store.rotate_checkpoint(auth(), bounded).unwrap_err().code,
            "checkpoint_budget"
        );
        assert_eq!(store.list_checkpoints().unwrap(), third.retained);
    }
}
