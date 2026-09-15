# flashtex-project-files

Original Rust project file layer for FlashTeX. Edition 2024; its only
dependencies are `unicode-normalization` and `libc` (the rooted file
operations' C bindings). Owner: `mac-project-files` (Claude Code subagent, parent `mac-claude-a`).

It answers four questions the single-file Mac shell cannot today: *which files
make up this project*, *what exactly is in them* (content identity), *how do I
save without losing someone else's write*, and *what changed on disk while the
editor had a buffer open* — plus a crash-recovery journal for unsaved buffers.

```sh
cargo test --manifest-path crates/project-files/Cargo.toml
cargo clippy --manifest-path crates/project-files/Cargo.toml --all-targets -- -D warnings
```

Tests use only temporary directories under the system temp dir; they never
touch the repository.

## API

### `ProjectPath` (`path.rs`)

`ProjectPath::normalize("./ch/../a.tex")` → `a.tex`. Always relative, forward
slashes, no `.`/empty segments. Rejected with `PathError`: absolute (`/`, `~`),
drive/colon, backslash, NUL/control characters, empty, and `..` that would
leave the root. `resolve_in(base_dir, raw)` resolves relative to a directory
inside the project. This is at least as strict as runtime-v1 ("project-relative,
no parent traversal") and transfer-v1 (no backslash/colon/NUL).

### Scanner (`scan.rs`)

`scan_references(text) -> Vec<Reference>` finds `\input`, `\include`,
`\bibliography` (one reference per comma item), `\addbibresource`,
`\includegraphics[..]{..}` and bare `\input name`. Each reference carries
`span` (whole command) and `argument_span` as zero-based, end-exclusive UTF-8
byte offsets, matching runtime-v1 `source`. `%` comments, `\verb`, and
`verbatim`/`comment`/`lstlisting`/`minted`/`Verbatim` bodies are skipped;
`\inputfoo` never matches `\input`. Arguments containing `\` or `#` are
returned with `literal == false` (they need macro expansion, which this crate
does not do).

### Graph (`graph.rs`)

```rust
let graph = ProjectGraph::discover(root, &ProjectPath::normalize("main.tex")?)?;
let graph = ProjectGraph::discover_with(root, &entry, &overlay)?; // unsaved buffers win
graph.files()         // ProjectFile { path, kind, source, text, sha256, bytes, references }
graph.edges()         // Edge { from, to, reference }
graph.diagnostics()   // Diagnostic { severity, message, path, span, argument_span, kind }
graph.documents()     // runtime-v1 [{path, text}] — Tex files, entry first, DFS order
graph.documents_including_bibliography()
graph.compile_payload(project_id, revision)   // Json object
graph.compile_envelope(id, project_id, revision) // one JSON Lines `compile` request
```

Resolution rules (TeX working-directory semantics: every name is resolved
against the project root, not the including file's directory):

| Reference | Candidates tried in order | Kind |
|---|---|---|
| `\input{x}`, `\include{x}` | `x.tex`, then `x` (just `x` if it already ends in `.tex`) | Tex, scanned recursively |
| `\bibliography{x}` | `x.bib` (or `x` if it ends in `.bib`) | Bibliography, loaded, not scanned |
| `\addbibresource{x}` | `x` if it has an extension, else `x.bib` | Bibliography |
| `\includegraphics{x}` | `x` if it has a known extension, else `x.pdf,.png,.jpg,.jpeg,.eps,.svg` | Graphic, hashed, not loaded |

Diagnostics (`DiagnosticKind`): `MissingFile{target, tried}` (error; warning
for graphics), `InvalidPath{target, error}` (escaping `..`, absolute, bad
characters), `EscapesRootViaSymlink`, `Cycle{chain}`, `UnresolvableReference`
(macro in argument, warning), `InvalidUtf8`, `ReadError`, `DepthExceeded`
(64 levels). Every reference diagnostic carries the referencing file plus
`span`/`argument_span`.

Ordering is deterministic: depth-first from the entry in source reference
order; a file reached twice (diamond) appears once, at its first visit; the
cycle-closing reference is recorded as an edge and a diagnostic but not
revisited. `DiscoverError` covers only the cases where nothing useful can be
built: root not a directory, entry missing/unreadable/not UTF-8.

### Content identity (`sha256.rs`, `revision.rs`)

`sha256(bytes)`, `sha256_hex(bytes)`, streaming `Sha256::update/finalize`,
`sha256_to_hex`, `sha256_from_hex`. Hand-written FIPS 180-4; tested against
"abc", "", the 56-byte two-block vector, one million `a`, and every chunking of
a 300-byte input.

`RevisionTracker`: `observe(path, text)` returns `(FileRevision{revision,
sha256, bytes}, changed)`; a file's revision starts at 1 and increments only
when its hash changes. `project_revision()` is 0 until something is observed
and then +1 per content change or `remove`; the same sequence of observations
always yields the same numbers. `observe_graph(&graph)` records every text
file of a discovery. Use `project_revision()` as the runtime-v1 `revision`.

### Rooted read and save (`save.rs`, `sys.rs`) — issue #18

```rust
pub struct ProjectRoot;                     // open directory handle
impl ProjectRoot {
    pub fn open(path: &Path) -> Result<ProjectRoot, SaveError>;
    pub fn path(&self) -> &Path;
    pub fn read(&self, path: &ProjectPath, limit: u64) -> Result<Option<RootedRead>, SaveError>;
    pub fn read_text(&self, path: &ProjectPath, limit: u64) -> Result<Option<(String, RootedRead)>, SaveError>;
    pub fn lock(&self) -> Result<ProjectLock<'_>, SaveError>;   // non-blocking flock(LOCK_EX)
    pub fn save(&self, path: &ProjectPath, bytes: &[u8], expected: Expected, force: bool) -> Result<SaveReceipt, SaveError>; // lock + save
    pub fn remove(&self, path: &ProjectPath) -> Result<bool, SaveError>;                                                    // lock + remove
}
pub struct ProjectLock<'a>;                 // released on drop
impl ProjectLock<'_> {
    pub fn root(&self) -> &ProjectRoot;
    pub fn save(&self, path: &ProjectPath, bytes: &[u8], expected: Expected, force: bool) -> Result<SaveReceipt, SaveError>;
    pub fn remove(&self, path: &ProjectPath) -> Result<bool, SaveError>;
}
pub fn save_atomic(root: &Path, path: &ProjectPath, text: &str, expected: Expected, force: bool) -> Result<SaveReceipt, SaveError>; // open + lock + save
pub fn save_atomic_bytes(root: &Path, path: &ProjectPath, bytes: &[u8], expected: Expected, force: bool) -> Result<SaveReceipt, SaveError>;

pub enum Expected { NewFile, Hash(Digest), Any }
pub struct SaveReceipt { path, bytes: u64, sha256: Digest, mtime: SystemTime, identity: FileIdentity }
pub struct RootedRead  { path, bytes: Vec<u8>, sha256, mtime, identity: FileIdentity, mode: u32 }
pub struct FileIdentity { dev: u64, ino: u64 }
pub enum SaveError { Conflict(Box<SaveConflict>), Refused(Refused), Io(io::Error), DirectorySync(io::Error) }
pub enum Refused { SymlinkComponent{component}, NotADirectory{component}, NotARegularFile{component},
                   EscapesRoot{component}, TooLarge{limit,size}, LockUnavailable{lock_path}, Unsupported }
pub struct SaveConflict { path, kind: SaveConflictKind, ours: Option<Digest>, theirs: Option<Digest>, mtime, size }
pub enum SaveConflictKind { ModifiedExternally, DeletedExternally, AlreadyExists, ModifiedDuringSave }
pub const LOCK_FILE: &str = ".flashtex/project.lock";
pub const DEFAULT_READ_LIMIT: u64 = 64 MiB;
```

**Path binding.** `ProjectRoot::open` opens the root directory itself with
`O_DIRECTORY|O_NOFOLLOW`, so a root whose final component is a symlink is
refused. **Symlinks among the root's ancestors are followed, by design:**
the parent of the root is opened by path with ordinary resolution, so for
`/Users/me/link/project` with `link -> /Volumes/work` the pinned root is
`/Volumes/work/project`. The caller chose that path, and symlinked home
directories, checkouts and mounts are common. The refuse-all-symlinks policy
applies *inside* the root, from the pinned descriptor onward; changing an
ancestor later does not move a root that is already open. Every read, save and
remove then walks the normalized `ProjectPath` one component at a time with
`openat(dirfd, component, O_DIRECTORY|O_NOFOLLOW)` from that handle, and
opens the final file with `openat(dirfd, name, O_NOFOLLOW)`. Any symlink —
parent directory or the file — is `Refused::SymlinkComponent`, with `force`
or without. Each walked directory's `..` is opened and its device/inode
compared with the handle it was reached from (`Refused::EscapesRoot` on
mismatch). Because `std` has no `openat` family, `sys.rs` calls
`openat`/`fstatat`/`renameat`/`unlinkat`/`mkdirat`/`flock` through the
`libc` crate, which supplies every symbol, flag, errno value and struct
layout per target; nothing is declared by hand. Rooted operations are enabled
on macOS and on Linux (glibc or musl); other targets get
`Refused::Unsupported` before anything is attempted. On macOS a symlink-to-directory opened this way
reports `ENOTDIR`; `sys::open_dir_at_nofollow` then classifies the entry with
`fstatat(AT_SYMLINK_NOFOLLOW)` (never a second open) so the refusal is
classified as a symlink on both platforms.

**Reads** are bounded: `read(path, limit)` refuses files larger than `limit`
(`Refused::TooLarge`) and non-regular files, and returns the bytes, hash,
mtime, identity and mode from the same open descriptor.

**Save sequence** (`ProjectLock::save`), all under the project lock:

1. Walk to the parent directory (missing directories are created with
   `mkdirat`, mode 0o755), refusing symlinks.
2. Observe the target with `O_NOFOLLOW`: identity, size, mtime, hash, mode.
   Unless `force`, compare with `expected` → `ModifiedExternally`,
   `DeletedExternally` or `AlreadyExists`, nothing written.
3. `openat(O_WRONLY|O_CREAT|O_EXCL|O_NOFOLLOW)` a temp file
   `.<name>.flashtex-tmp-<pid>-<n>` in the same directory, write, `fsync`,
   `fchmod` to the existing mode.
4. Re-observe the target. If it is now a symlink → `Refused` (temp removed).
   If, unless `force`, anything changed since step 2 (a file appeared,
   disappeared, or its identity/size/mtime/hash moved) →
   `Conflict{ModifiedDuringSave, ours: hash we wrote, theirs: observed}`,
   temp removed, target untouched. This is where `Expected::NewFile` refuses
   to clobber a target created meanwhile.
5. Classify the target once more with `fstatat(AT_SYMLINK_NOFOLLOW)`: a
   symlink or special file is refused (with `force` or without), and unless
   `force` a different file than step 4 saw is `ModifiedDuringSave`.
   - **Absent target (fail-closed no-clobber):** install the temp file with
     `renameat2(RENAME_NOREPLACE)` (Linux) / `renameatx_np(RENAME_EXCL)`
     (macOS). Where the filesystem does not support that, use
     `linkat(temp, target)` — which also fails with `EEXIST` atomically —
     then `unlinkat(temp)`. An entry that appeared after the check is never
     replaced: it is refused if it is a symlink or special file, otherwise
     `ModifiedDuringSave` unless `force`. If neither primitive is supported,
     a non-forced save fails with an `Unsupported` I/O error and writes
     nothing; only `force` falls back to a plain `renameat`.
   - **Existing target:** `renameat` temp over target.

   Then `fsync` the directory. A directory
   fsync failure is `SaveError::DirectorySync` — a hard error; the rename has
   already happened and durability is unknown, so re-read before trusting.
6. Re-open the target with `O_NOFOLLOW`; its device/inode must equal the temp
   file's and its bytes must hash to what was written, else
   `Conflict{ModifiedDuringSave}`. The receipt carries the verified identity,
   size, hash and mtime.

**Lock.** `ProjectRoot::lock` takes a non-blocking advisory `flock(LOCK_EX)`
on `<root>/.flashtex/project.lock` (created if missing) and returns
`Refused::LockUnavailable{lock_path}` if any other open file description —
another process, or another handle in this one — holds it. The lock is held
for the whole sequence above and released on drop. `ProjectRoot::save` and
`save_atomic` lock per call; an editor that wants to serialize many
operations (saves, journal writes, removals) holds one `ProjectLock` and
passes it around.

### Concurrency and durability contract

- **Supported serialization is the project lock.** Two FlashTeX writers
  (threads or processes) that both use this crate never interleave inside a
  save: the second gets `LockUnavailable` and must retry or report. Within
  the contract, a save is a compare-and-replace: the hash check, temp write,
  rename and directory fsync happen with no other in-contract writer able to
  act.
- **Writers that do not take the lock are out of contract.** Neither a
  path-only check nor a double hash check can *prevent* such a writer from
  landing between step 4 and step 5 (a few microseconds) or immediately
  after step 5. The crate does not claim otherwise. What it does: it
  *detects* interference at step 4 (pre-rename re-verification of identity,
  size, mtime and content hash) and at step 6 (post-rename identity and hash
  verification), and reports `Conflict{ModifiedDuringSave}` whenever it is
  observed. A writer that lands exactly inside the step-4→step-5 window with
  identical size and an mtime inside the filesystem's timestamp resolution
  is not detected until the next `Snapshot::diff`. The racing test in
  `tests/rooted.rs` exercises this with a thread swapping the target between
  a file and a symlink: every save ends `Ok`, `Refused` or `Conflict`, and
  the outside file is never touched.
- **Durability.** Bytes are `fsync`ed before the rename and the directory is
  `fsync`ed after it; both failures are hard errors (`Io` before rename, with
  the temp removed; `DirectorySync` after rename, with the file in place but
  durability unknown). Rename atomicity is the filesystem's: within one
  directory on APFS, HFS+, ext4 and XFS a `rename(2)` is atomic for other
  readers. Network and FAT volumes may not honor this. No power-loss test has
  been run; a receipt means the syscalls reported success, not that the media
  has been verified.
- **No claim of unconditional protection.** Out-of-contract writers, bind
  mounts inside the root, `chroot`-escaping hard links, and privileged
  processes are outside what this API can defend; it confines *its own*
  reads and writes to the selected root and reports what it can observe.

### External-change detection (`watch.rs`)

```rust
let snap = Snapshot::take(root, graph.text_paths())?;   // hashes every path
let diff = snap.diff()?;                                  // rehash only if mtime/size moved
diff.changes  // ExternalChange { path, kind: Created|Modified|Deleted, before, after }
diff.root_replaced  // the root path no longer names the directory `take` pinned
diff.conflicts(&dirty_paths)  // Conflict { path, kind, local_dirty, before, after }
snap.record_own_write(&receipt.path, receipt.bytes, receipt.mtime, receipt.sha256);
```

`ConflictKind`: `ModifiedExternally` (disk changed, buffer clean — safe to
reload), `DeletedExternally` (with `local_dirty` telling you whether unsaved
edits would be lost), `Both` (disk changed *and* the buffer has unsaved edits —
the three-way case). Creations are changes but not conflicts. A file rewritten
with identical bytes is not reported. Missing paths (e.g. a not-yet-created
include) are tracked so their creation is reported.

**Pinned root.** `Snapshot::take` opens the root once and keeps that
descriptor (clones and the snapshot a `Diff` carries forward share it).
`track` and `diff` read through it and never re-resolve the root path, so a
directory renamed into the root's place is never stat'ed or hashed. `diff`
compares the pinned descriptor's device/inode with whatever the path names
now; if the root was renamed, removed or replaced (by a directory or a
symlink) it sets `root_replaced`, while `changes` keep describing the pinned
directory. A root that did not exist at `take` is opened and pinned the
first time it exists.

`Poller` wraps a snapshot: `poll()` returns changes and advances (a replaced
root is an `io::Error` wrapping `RootReplaced`, and the poll does not
advance);
`run(interval, deadline, |result| keep_going)` loops on the calling thread.
There is deliberately no FSEvents dependency: the native app can call
`Snapshot::diff` from an FSEvents callback later and keep the same conflict
semantics.

### Crash recovery (`recovery.rs`)

```rust
let root = ProjectRoot::open(root_path)?;
let journal = RecoveryJournal::new(&root);
let lock = root.lock()?;
let entry = journal.record(&lock, &path, unsaved_text, Some(base_hash))?;  // rooted, atomic
let listing = journal.list()?;                 // entries sorted by path + malformed files
let check = journal.check(&entry)?;            // CurrentState::{Missing, MatchesBase, MatchesJournal, Diverged(hash)}, safe
journal.restore_to_disk(&lock, &entry, force)?; // same conflict rules as ProjectLock::save; discards on success
journal.discard(&lock, &path)?;
```

Files live at `<root>/.flashtex/recovery/<sha256_hex(path)>.json`:
`{schema_version:1, path, text, text_sha256, base_sha256|null,
saved_at_unix_ms}`. Entries are written through `ProjectLock::save` (same
walk, lock, fsync and verification) and read through `ProjectRoot::read`
(symlinks refused, 64 MiB bound). On read, `text_sha256` is verified, so a
torn or truncated entry is reported as malformed rather than restored.
Malformed files are listed, never deleted. `restore_to_disk` refuses (returns
the `SaveConflict`) when the disk has diverged from `base_sha256`, or when a
never-saved file now exists, unless `force`. A file that already equals the
journal text is treated as restored without a write.

Recommended editor policy: record every N seconds while dirty and on focus
loss; discard on successful save; on launch, `list()` and present each entry
with its `check()` result.

## Guarantees

- Paths in the graph, documents, receipts and journal are normalized
  `ProjectPath`s. Every write and rooted read is confined to the opened root
  directory handle: no symlink at any component is followed, and a
  refusal happens before any byte is written.
- Discovery output (file order, edges, diagnostics, documents, compile
  envelope) is a pure function of the file tree plus overlay.
- All byte spans are UTF-8 byte offsets into the exact text the graph holds
  (overlay text when supplied, disk bytes otherwise).
- SHA-256 output matches the FIPS vectors; `text_sha256`/`SaveReceipt.sha256`
  are hashes of the exact bytes written, re-read from disk after the rename.
- A save either fully replaces the target with the new bytes or leaves the
  previous file intact; a refused or conflicted save writes nothing to the
  target and removes its temp file. Permissions of an existing target are
  preserved.
- Saves by in-contract writers are serialized by the project lock.
- Snapshot/diff never misses a content change whose mtime or size changed;
  a change that leaves both identical is caught on the next hash (see below).
- The recovery journal never restores over diverged content without `force`
  and never reports a corrupted entry as valid.

## Non-guarantees

- **Out-of-contract writers are detected, not prevented** (see the contract
  above). `ModifiedDuringSave` is a report, not a rollback.
- **Rename atomicity and fsync semantics are the filesystem's.** Network and
  FAT volumes may not honor them; no power-loss testing has been done.
- **mtime granularity.** A same-size rewrite within the filesystem's mtime
  resolution (nanoseconds on APFS, coarser elsewhere) is not rehashed by
  `Snapshot::diff`. Callers can force a rehash with a fresh `Snapshot::take`.
- **Rename over, and unlink of, an existing entry are checked, not
  compare-and-swap.** Graph discovery, `Snapshot`, the save-path
  normalization fallback and the recovery journal listing all stat and list
  through the pinned root descriptor, never a path string. A save classifies
  an existing target with `fstatat(AT_SYMLINK_NOFOLLOW)` immediately before
  `renameat`, and `remove` does the same before `unlinkat`. POSIX has no
  "rename over / unlink only if this is still that inode", so an entry
  swapped in between the check and the call, by a process that can write the
  project directory, is still replaced or removed. That is the whole
  residual: the effect is limited to replacing or removing that one
  directory entry inside the pinned directory. Neither call follows a
  symlink (the link itself is replaced or removed; its target is never
  opened, written or deleted), neither can replace or remove a directory,
  and nothing outside the pinned directory is affected. A save's
  post-rename verification still confirms the saved file is what sits at
  the name. Creating an absent target has no such window (see step 5). An
  exchange-then-verify rename (`RENAME_EXCHANGE`/`RENAME_SWAP` plus a swap
  back) was considered and not adopted: removing the displaced entry has the
  same check-then-unlink window, so it moves the residual instead of closing
  it.
- **Symlinks above the root are followed.** Only the root's own final
  component and everything inside it are refused as symlinks (see *Path
  binding*). Choosing a path through a symlinked ancestor chooses the
  directory it leads to.
- **Special files are classified before they are opened, not atomically.**
  An entry is `fstatat(AT_SYMLINK_NOFOLLOW)`-classified on the pinned
  directory descriptor and opened only if it is a regular file. POSIX cannot
  make the classification and the open one step, so the entry can be swapped
  in between. A symlink swapped in is still refused by `O_NOFOLLOW` and never
  followed. A FIFO swapped in does not block the open, because of
  `O_NONBLOCK`, and the `fstat` of the opened descriptor refuses it. A
  device node swapped in is opened non-blocking and then refused by that
  `fstat`, and nothing is read from it. Not hanging and not triggering a
  driver's open side effect are **best-effort** for devices: they rest on
  `O_NONBLOCK`, the pre-open `fstatat` and the post-open `fstat`, and
  whether the driver honours `O_NONBLOCK` is up to the driver.
- **Race tests.** Each check-then-use window in `save.rs` calls a hidden,
  thread-local test hook (`save::race_hook`, not API). The tests swap the
  entry from inside the hook, so the worst interleaving is exercised
  deterministically on every run. The older timing-based stress tests are
  kept as extra coverage.
- **Not the compiler.** The scanner does no macro expansion, no catcode
  changes, no `\import`/`\subfile`/`\InputIfFileExists`, and does not follow
  references inside `\newcommand` bodies or conditionals. Arguments containing
  macros are reported, not resolved. `\include` inside a `\includeonly`
  exclusion is still discovered.
- Graphics are hashed for identity but never parsed or validated.
- The poller is blocking and single-threaded by design; scheduling is the
  caller's.
- `.flashtex/` (lock file and journal) lives inside the project root and is
  not hidden from other tools; the native app should add it to VCS ignore
  rules if desired.
- Targets other than macOS and Linux (glibc or musl) have no rooted file
  operations (`Refused::Unsupported`); the crate itself is Unix-only.

### JSON Lines helper (`src/bin/flashtex-project-files.rs`)

`flashtex-project-files --root DIR` hosts the rooted primitives above for a
native consumer over private pipes (one process per project root, replies in
request order, one JSON object per line, 12 MiB line bound). Protocol
`project-files-v1`:

| Request | Payload |
|---|---|
| `{"id","operation":"ping"}` | `{"protocol","root","pid"}` |
| `{"id","operation":"read","path"}` | `{"path","exists","text"?,"sha256"?,"bytes"?,"mtime_unix_ms"?}` |
| `{"id","operation":"status","path","expected_sha256"?}` | `{"path","exists","state":"unchanged"\|"modified"\|"deleted"\|"created",…}` relative to `expected_sha256` (`null`: caller expects no file) |
| `{"id","operation":"save","path","text","expected":"new"\|"any"\|hex,"force"?}` | `{"outcome":"saved","receipt":{"path","bytes","sha256","mtime_unix_ms"}}` or `{"outcome":"conflict","conflict":{"path","kind","ours"?,"theirs"?,"mtime_unix_ms"?,"size"?}}` |

Errors are `{"id","error":{"code","message"}}`: `invalid_request`,
`invalid_path`, `refused` (symlink component, escapes root, not a regular
file, too large, lock held, unsupported target), `invalid_utf8`, `io`,
`directory_sync`, `unsupported_operation`, `line_too_long`. A save conflict
is a payload, never an error, because the consumer must show it and keep
its buffer. `path` is a `ProjectPath` relative to `--root`; `--root` itself
must be a real directory (the Mac shell resolves symlinks in the directory
it derives from the opened file before launching the helper). The Mac
consumer is `apps/mac/Sources/FlashTeXMac/DocumentFilesClient.swift` /
`DocumentFiles.swift` (owner `mac-document-files`).

## Relationship to sibling crates (main at c89ca86)

- `crates/project-index` (lexical labels/citations/commands navigation) never
  reads files; it takes `(path, revision, text)` per document. `documents()`
  plus `RevisionTracker::file(path).revision` is exactly that input, and the
  normalized `ProjectPath` strings satisfy its path rule (no `.`/`..`
  components remain after normalization).
- `crates/edit-ledger` is a private per-document durable store
  (`document.json` + applied capture-edit IDs, exclusive file lock). It is the
  authority for a document's *live* text and receipts; a `.tex` file is an
  export of it. This crate is the `.tex`-side layer the ledger explicitly
  leaves to the Mac owner: discovering the rest of the project, exporting the
  ledger's text with `save_atomic` (hash-checked against external edits, which
  the ledger does not watch), and reporting external changes/deletions via
  `Snapshot::diff`. Where a document is opened in the ledger, the
  `RecoveryJournal` here is redundant for it; the journal remains useful for
  buffers that are not ledger-backed (included files, `.bib`).
- `crates/document-runtime` submits complete unsaved project snapshots per
  revision; `compile_payload(project_id, tracker.project_revision())` is that
  snapshot.

## Native project integration proposal (follow-up 2)

Current state on `agent/mac-claude-a/mac-shell` (`DocumentFiles.swift`):
one `.tex` file is opened with `String(contentsOf:)`, becomes `main.tex` in
the compile request regardless of its real name, `isDirty` compares strings,
and `saveTex` uses `String.write(atomically:)` with no hash or conflict check.

Proposed replacement, coordinated with the parent (`mac-claude-a`) before any
`Package.swift`/shared file change:

1. **Project root, not file.** "Open" picks a `.tex` file; the app derives
   `root = fileURL.deletingLastPathComponent()` and `entry = fileName` (a
   later "Open Folder" can ask for the entry). Security-scoped bookmarks cover
   the root.
2. **A Rust `project-files` service over the existing JSON Lines pattern**
   (the same shape as the bridge process in transfer-v1), or an FFI surface if
   the parent prefers in-process: `project_open{root, entry}`,
   `project_discover{overlay:[{path,text}]}` → `{files, diagnostics, documents,
   revision}`, `project_save{path, text, expected_sha256|null, force}` →
   `SaveReceipt` or `SaveConflict`, `project_poll{}` → `[ExternalChange]`,
   `recovery_list/record/check/restore/discard`. Every payload here is already
   producible from the crate's public types; a thin JSON adapter is the only
   new code.
3. **Compile requests come from `documents()`.** The shell stops synthesizing a
   one-element `documents` array; `compile_payload(project_id,
   tracker.project_revision())` is the request body, so the compiler (issue
   #8 / FT-002 file-aware `\input`) receives every reachable file with its
   real path and the editor's unsaved text via `Overlay`. Diagnostics from
   discovery (missing include, cycle) are shown in the same diagnostics list
   as compiler output, mapped through `span` with the existing UTF-8 → UTF-16
   conversion.
4. **Save path.** `saveTex` → `project_save(expected: lastReceipt.sha256)`.
   On `SaveConflict` show ours/theirs hashes, mtime and size with
   "Overwrite" (force) / "Reload" / "Save As" actions; never auto-force.
   The receipt hash becomes the new baseline and is fed to
   `record_own_write` so the poller stays quiet.
5. **External changes.** A 2 s poll on a background queue (or FSEvents later
   feeding `Snapshot::diff`). `ModifiedExternally` with a clean buffer reloads
   silently and recompiles; `Both`/`DeletedExternally` present the conflict
   sheet. This replaces the string-compare `isDirty`.
6. **Crash recovery.** While dirty, `record` every 5 s and on resign-active;
   `discard` after a successful save; on launch, `list()` → recovery sheet
   with `check()` results ("safe to restore" / "file changed since").
7. **Multi-buffer editor** follows naturally: the overlay is the set of open,
   dirty buffers keyed by `ProjectPath`; the transfer-v1 bridge's `document_*`
   messages already use the same path/revision/hash vocabulary.

Sequencing: (a) parent agrees on process-vs-FFI and message names, (b) I add
the JSON adapter and a `flashtex-project-files` binary under this crate,
(c) parent wires `DocumentFiles.swift` behind a feature flag, (d) native
acceptance on this Mac: open a nested project, edit an included file, save
with an external modification, recover after a forced kill.

Status: (a) decided as a child process (parent dispatch to
`mac-document-files`); (b) done for `read`/`status`/`save` (binary above);
(c) done in `DocumentFiles.swift` with a direct-Foundation fallback when no
binary is found; `project_discover`, `project_poll` and the recovery
operations are not yet exposed over the wire.
