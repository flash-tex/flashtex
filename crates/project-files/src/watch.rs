//! External-change detection by polling: a [`Snapshot`] records mtime, size
//! and SHA-256 per tracked path; [`Snapshot::diff`] rehashes only entries
//! whose mtime or size moved and reports created/modified/deleted paths.
//!
//! There is no FSEvents dependency. The native app may later drive
//! [`Snapshot::diff`] from an FSEvents callback instead of the [`Poller`];
//! the comparison logic is the same.

use std::collections::{BTreeMap, BTreeSet};
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

use crate::path::ProjectPath;
use crate::save::{ProjectRoot, Refused, SaveError};
use crate::sha256::{Digest, sha256};

/// On-disk identity of one file at snapshot time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileState {
    pub size: u64,
    pub mtime: SystemTime,
    pub sha256: Digest,
}

/// Recorded state for a set of project paths (missing paths are tracked as
/// `None` so their later creation is reported).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    root: PathBuf,
    entries: BTreeMap<ProjectPath, Option<FileState>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeKind {
    /// The path did not exist at snapshot time and does now.
    Created,
    /// Content hash differs from the snapshot.
    Modified,
    /// The path existed at snapshot time and is gone.
    Deleted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalChange {
    pub path: ProjectPath,
    pub kind: ChangeKind,
    pub before: Option<FileState>,
    pub after: Option<FileState>,
}

/// Result of a diff: what changed, plus a snapshot of the observed state to
/// carry forward.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diff {
    pub changes: Vec<ExternalChange>,
    pub snapshot: Snapshot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictKind {
    /// Disk content changed; the local buffer is clean (safe to reload).
    ModifiedExternally,
    /// The file was deleted on disk (the `local_dirty` flag says whether
    /// unsaved edits would be lost).
    DeletedExternally,
    /// Disk content changed *and* the local buffer has unsaved edits.
    Both,
}

/// A change that needs a user decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Conflict {
    pub path: ProjectPath,
    pub kind: ConflictKind,
    pub local_dirty: bool,
    pub before: Option<FileState>,
    pub after: Option<FileState>,
}

impl Snapshot {
    /// Records the current state of `paths` under `root`, hashing each file.
    pub fn take<'a>(
        root: &Path,
        paths: impl IntoIterator<Item = &'a ProjectPath>,
    ) -> io::Result<Snapshot> {
        let rooted = RootHandle::open(root)?;
        let mut entries = BTreeMap::new();
        for p in paths {
            entries.insert(p.clone(), rooted.read_state(p, None)?);
        }
        Ok(Snapshot {
            root: root.to_path_buf(),
            entries,
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn paths(&self) -> impl Iterator<Item = &ProjectPath> {
        self.entries.keys()
    }

    pub fn state(&self, path: &ProjectPath) -> Option<&FileState> {
        self.entries.get(path).and_then(Option::as_ref)
    }

    /// Adds a path to track (hashes it now).
    pub fn track(&mut self, path: &ProjectPath) -> io::Result<()> {
        let state = RootHandle::open(&self.root)?.read_state(path, None)?;
        self.entries.insert(path.clone(), state);
        Ok(())
    }

    pub fn untrack(&mut self, path: &ProjectPath) {
        self.entries.remove(path);
    }

    /// Records that we ourselves just wrote `path` (from a
    /// [`SaveReceipt`](crate::save::SaveReceipt)), so the write is not
    /// reported as external.
    pub fn record_own_write(
        &mut self,
        path: &ProjectPath,
        size: u64,
        mtime: SystemTime,
        sha256: Digest,
    ) {
        self.entries.insert(
            path.clone(),
            Some(FileState {
                size,
                mtime,
                sha256,
            }),
        );
    }

    /// Compares disk against this snapshot. Only files whose size or mtime
    /// changed are rehashed; a file rewritten with identical content is not
    /// reported.
    pub fn diff(&self) -> io::Result<Diff> {
        let mut changes = Vec::new();
        let mut entries = BTreeMap::new();
        let rooted = RootHandle::open(&self.root)?;
        for (path, before) in &self.entries {
            let after = rooted.read_state(path, *before)?;
            let kind = match (before, &after) {
                (None, None) => None,
                (None, Some(_)) => Some(ChangeKind::Created),
                (Some(_), None) => Some(ChangeKind::Deleted),
                (Some(b), Some(a)) => (b.sha256 != a.sha256).then_some(ChangeKind::Modified),
            };
            if let Some(kind) = kind {
                changes.push(ExternalChange {
                    path: path.clone(),
                    kind,
                    before: *before,
                    after,
                });
            }
            entries.insert(path.clone(), after);
        }
        Ok(Diff {
            changes,
            snapshot: Snapshot {
                root: self.root.clone(),
                entries,
            },
        })
    }
}

impl Diff {
    /// Turns changes into explicit conflicts given the set of paths whose
    /// local buffers have unsaved edits. Creations are not conflicts.
    pub fn conflicts(&self, dirty: &BTreeSet<ProjectPath>) -> Vec<Conflict> {
        self.changes
            .iter()
            .filter_map(|c| {
                let local_dirty = dirty.contains(&c.path);
                let kind = match c.kind {
                    ChangeKind::Created => return None,
                    ChangeKind::Deleted => ConflictKind::DeletedExternally,
                    ChangeKind::Modified if local_dirty => ConflictKind::Both,
                    ChangeKind::Modified => ConflictKind::ModifiedExternally,
                };
                Some(Conflict {
                    path: c.path.clone(),
                    kind,
                    local_dirty,
                    before: c.before,
                    after: c.after,
                })
            })
            .collect()
    }

    pub fn is_empty(&self) -> bool {
        self.changes.is_empty()
    }
}

/// The project root, pinned once per [`Snapshot::take`], [`Snapshot::track`]
/// or [`Snapshot::diff`]. Every tracked file is stat'ed and hashed through
/// the same rooted, symlink-refusing walk that reads and saves use
/// ([`ProjectRoot`]), never through a path string: a tracked path that is,
/// or lies under, a symlink is never followed, stat'ed or hashed, and
/// nothing but a regular file is opened. On a target without rooted file
/// operations this fails closed with an `Unsupported` error.
enum RootHandle {
    Open(ProjectRoot),
    /// The root directory itself does not exist: every path is absent.
    Missing,
}

impl RootHandle {
    fn open(root: &Path) -> io::Result<RootHandle> {
        match ProjectRoot::open(root) {
            Ok(r) => Ok(RootHandle::Open(r)),
            Err(SaveError::Io(e)) if e.kind() == io::ErrorKind::NotFound => Ok(RootHandle::Missing),
            Err(e) => Err(save_error_to_io(e)),
        }
    }

    /// Reads the state of `path`, reusing `previous`'s hash when size and
    /// mtime are unchanged. `Ok(None)` if the path does not exist *or* is not
    /// something the rooted reader will open: a symlink (the file itself or
    /// an ancestor directory, wherever it points), a non-directory ancestor,
    /// a directory whose `..` no longer leads back, or a non-regular file.
    /// Such a path is reported like a deleted file, and its contents are
    /// never read.
    fn read_state(
        &self,
        path: &ProjectPath,
        previous: Option<FileState>,
    ) -> io::Result<Option<FileState>> {
        let RootHandle::Open(root) = self else {
            return Ok(None);
        };
        let (mut file, meta) = match root.open_regular(path) {
            Ok(Some(opened)) => opened,
            Ok(None) => return Ok(None),
            Err(SaveError::Refused(
                Refused::SymlinkComponent { .. }
                | Refused::NotADirectory { .. }
                | Refused::NotARegularFile { .. }
                | Refused::EscapesRoot { .. },
            )) => return Ok(None),
            // A missing intermediate directory: the path does not exist.
            Err(SaveError::Io(e)) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(save_error_to_io(e)),
        };
        let size = meta.len();
        let mtime = meta.modified()?;
        if let Some(prev) = previous
            && prev.size == size
            && prev.mtime == mtime
        {
            return Ok(Some(prev));
        }
        let mut bytes = Vec::with_capacity(size as usize);
        io::Read::read_to_end(&mut file, &mut bytes)?;
        // Re-read metadata (of the same descriptor) so a write racing with
        // our read is caught next time.
        let meta2 = file.metadata()?;
        Ok(Some(FileState {
            size: meta2.len(),
            mtime: meta2.modified()?,
            sha256: sha256(&bytes),
        }))
    }
}

fn save_error_to_io(e: SaveError) -> io::Error {
    match e {
        SaveError::Io(e) | SaveError::DirectorySync(e) => e,
        SaveError::Refused(Refused::Unsupported) => io::Error::new(
            io::ErrorKind::Unsupported,
            "rooted file operations are not available on this target",
        ),
        other => io::Error::other(other.to_string()),
    }
}

/// Polling helper: holds the latest snapshot and reports changes on demand
/// or on a fixed interval. Single-threaded and blocking; the caller decides
/// which thread runs it.
#[derive(Debug)]
pub struct Poller {
    snapshot: Snapshot,
}

impl Poller {
    pub fn new(snapshot: Snapshot) -> Self {
        Self { snapshot }
    }

    pub fn snapshot(&self) -> &Snapshot {
        &self.snapshot
    }

    pub fn snapshot_mut(&mut self) -> &mut Snapshot {
        &mut self.snapshot
    }

    /// Diffs once and advances the internal snapshot.
    pub fn poll(&mut self) -> io::Result<Vec<ExternalChange>> {
        let diff = self.snapshot.diff()?;
        self.snapshot = diff.snapshot;
        Ok(diff.changes)
    }

    /// Polls every `interval` until `on_poll` returns `false` or `deadline`
    /// passes. Errors are passed to the callback as `Err`.
    pub fn run(
        &mut self,
        interval: Duration,
        deadline: Option<Instant>,
        mut on_poll: impl FnMut(io::Result<Vec<ExternalChange>>) -> bool,
    ) {
        loop {
            if deadline.is_some_and(|d| Instant::now() >= d) {
                return;
            }
            if !on_poll(self.poll()) {
                return;
            }
            std::thread::sleep(interval);
        }
    }
}
