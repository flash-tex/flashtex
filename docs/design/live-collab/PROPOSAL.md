# Live collaboration — design proposal (LIVE-COLLAB-DESIGN)

Status: **adopted** 2026-10-04 (the owner asked to start building Live Share; P0 is
lane LIVE-SHARE-P0, see §10). Author: mac-claude-a (mac-m1max-a), 2026-10-02. Requested by the owner: Overleaf-style live collaboration, where several
people edit one LaTeX project at once, see each other's cursors and selections, and
all get the compiled preview.

`docs/design/engine-v2/DESIGN.md` remains the single source of truth and is owned by
the Commander. This document does not change it. Section 9 lists what DESIGN.md
would need to say if the owner adopts this.

---

## 0. Recommendation in brief

1. **Sync model.** Use a sequence CRDT per text file, of the YATA/Fugue family with
   run-length encoding, plus a small map CRDT for the project's file tree. Binary
   assets are content-addressed blobs. Do not use OT, and do not use a purely
   server-authoritative model.
2. **Implementation.** Write a pure-Swift, platform-free `FlashTeXCollabCore` target,
   shared by the Mac and the iPad the same way `FlashTeXEditorCore` is shared. The
   wire format is a written contract (`collab-v1`). The existing MIT Rust crate
   `crates/collaboration-core` grows into the reference implementation and the
   differential-fuzz oracle. The reason is that `scripts/check-license-boundary.sh`
   (B2/B3) forbids any binary target, xcframework or cargo step under `apps/ios`,
   which rules out Yrs or Automerge on the iPad unless the owner amends that rule
   (§8, Q1).
3. **Topology.** The inviting Mac is the session **hub**: a star on the LAN that
   reuses the nearby link's Bonjour, Network.framework listener and caps. The hub
   is the durable owner, but it is not a sequencer, so guests keep editing through
   a hub outage. An end-to-end-encrypted internet relay is a later phase.
   CloudKit is not used for real-time sync.
4. **Compile.** Every Mac compiles locally from the converged state with its own
   `flashtex-host`. A Mac in the session streams its `display-list-v3` frames to
   iPads, because an iPad cannot run the GPL engine (no subprocesses on iOS, and no
   GPL linking). Determinism is enforced by session pins and checked by comparing
   per-page content hashes.
5. **Undo.** Undo is local-only and expressed in CRDT identities. Remote edits never
   enter a local undo stack. The edit ledger stays the single-writer durable store
   on each Mac, and the converged text is fed to it through the existing
   `replace_document` route.

---

## 1. Goals, non-goals, MVP scope

### 1.1 Goals

- G1. Two to five people edit the same project at once and always converge to
  byte-identical sources, whatever the order or duplication of delivery and
  through disconnects and reconnects.
- G2. Local typing latency does not regress. The collaboration layer adds
  ≤ 1 ms p95 per keystroke to the app-side budget of DESIGN.md §1.2 (app ≤ 4 ms
  under O1's proposed split) on a 1 MB file.
- G3. A remote keystroke reaches every other editor within ≤ 50 ms p95 on a LAN.
- G4. Presence: everyone sees everyone else's caret, selection, name and colour,
  and the file and preview page each person is on. A follow mode is available.
- G5. Everyone gets the compiled preview. Macs compile locally; iPads view a
  Mac's preview.
- G6. Multi-file projects, figures (binary assets) and renames all work.
- G7. The licence boundary of DESIGN.md §3 is unchanged: no GPL code in the MIT
  app or on the iPad, and no new copyleft dependency.

### 1.2 Non-goals (for now)

- Comments, suggestion or track-changes mode, and a per-author history browser.
  These are later and can be layered on the CRDT's author attribution.
- A web client, more than five collaborators, or public share links.
- Merging concurrent edits *inside* a binary file. Last writer wins (§2.6).
- A shared compile server as the default (§5.2).
- Git or Dropbox integration, Windows and Linux clients (DESIGN.md §16 defers
  them), and XeLaTeX/LuaLaTeX-specific font syncing.
- Roles beyond `edit` and `view`.

### 1.3 MVP scope

One project; two to five participants; any mix of Macs and iPads, with at least one
Mac (the hub); same LAN; online editing with short offline gaps (≤ 10 min)
tolerated. Text files are `.tex`, `.sty`, `.cls`, `.bib`, `.bst`, `.toml` and plain
text; figures are synced as blobs. The iPad edits text and views the preview. It
cannot compile, and it cannot host.

---

## 2. Sync model

### 2.1 What exists today

| Piece | What it is | Relevance |
|---|---|---|
| `crates/collaboration-core` (MIT, no deps) | A YATA-style sequence CRDT: `OpId {counter, replica}`, `OpPayload::Insert {left, right, value: char}` / `Delete {target}`, `Document::apply` with duplicate detection and `IdConflict`, `PendingOps` for out-of-order delivery, `Checkpoint` v2 wire format. Tests: `convergence.rs`, `adversarial.rs`, `interrupted_delivery.rs`, `id_reuse_divergence.rs`, `checkpoint_equivalence.rs`. | The right algorithm family, with good adversarial tests. Not usable as-is for projects: one `Element` per `char` in a `Vec`, `position_of` is a linear scan, the bound is `DEFAULT_MAX_ELEMENTS = 200_000` (a 200 KB thesis chapter plus tombstones exceeds it), there are no relative positions for cursors, and no consumer links it. |
| `crates/edit-ledger` (MIT) | Single-writer durable `document.json` per document: revision + SHA-256 guard, fsync'd atomic rename, receipts, grouped edits and **durable undo/redo** (`history.rs`, `GroupedEdit`, `MAX_HISTORY_ENTRIES = 256`), recovery. Bounds: `MAX_DOCUMENT_BYTES` 8 MiB, `MAX_REPLACEMENT_BYTES` 64 KiB. | Stays each Mac's durable store. It is explicitly *not* a merge engine (its README and collaboration-core's header both say so). |
| `flashtex-preview-controller` route (`ShellModel+Controller.swift`) | Submits the whole active buffer as `edit {expected_revision, expected_sha256, text}`, one in flight, newest coalesced; `ControllerState.durable[path]` holds the durable revision. | The collaboration layer feeds converged text through exactly this route. |
| `crates/project-files` (MIT) | Project graph, `ProjectPath` normalisation, per-file `RevisionTracker` (SHA-256), atomic saves with hash-checked conflict refusal (`save.rs`), external-change `watch`, crash `recovery`. | It supplies path normalisation, file identity and the watcher; collaboration supplies the merge. |
| Mac editor (`SourceEditorView.swift`) | An `NSTextView` with `allowsUndo = true`; user edits flow `shouldChangeTextIn` → `commitUserChange` → `parent.text` → `ShellModel.updateActiveText` (bumps `editorRevision`, calls `EngineV3Session.textChanged`). Model → view sync is **`tv.string = text`** (a whole-buffer reset). Programmatic edits use `PendingEdit` / `applyPendingEdit` with `programmaticChanges` guards. | The whole-buffer reset is unusable for remote edits, because it drops the caret, undo and IME state. Remote edits need a ranged path (§2.4). |
| iPad editor (`EditorTextStorage.swift`, `EditorController.swift`, `PadDocument`) | `NSTextStorage` subclass with incremental highlighting and an `editSerial`; `PadDocument {path, text, revision}`; one local buffer, **not synced** (apps/ios/README.md: "transfer-v1 carries captures, not documents"). | Needs a multi-file model and a CRDT binding. |

### 2.2 Options

| | Text CRDT (YATA/Fugue, RGA, …) | OT (Jupiter/ShareJS, as early Overleaf) | Server-authoritative (lock-step via hub) |
|---|---|---|---|
| Needs a sequencer | No | Yes (the server) | Yes |
| Guest edits while hub is away | Yes, merges later | Only with buffered ops rebased on reconnect; long gaps are fragile | No |
| Hub/relay logic | Dumb forwarding plus storage | Must transform | Must validate and order |
| Cursor stability | Free: anchor to an `OpId` | Cursors must be transformed | Cursors must be transformed |
| Local undo with interleaved remote edits | Natural: undo the ids you created | Needs inverse transforms; well known to be hard | Hard |
| Memory | Tombstones; needs RLE and compaction | Minimal | Minimal |
| Fit with the edit ledger | Converged text replayed as ordinary edits | Same | Natural (single writer) |
| Correctness argument | Commutativity, checkable by fuzzing | TP1/TP2 proofs; P2P OT is notoriously error-prone | Trivial |

**Recommendation: CRDT.** It is the only option that survives hub restarts,
Wi-Fi drops and a later relay without moving logic into the server. It also
anchors presence and undo in stable identities. The team already has a tested
YATA implementation and its adversarial fixtures to start from.

### 2.3 Library choice

| Option | Licence | Mac | iPad | Verdict |
|---|---|---|---|---|
| **A. Own Swift core (`FlashTeXCollabCore`)**, an algorithm port of `collaboration-core` with RLE runs and a B-tree index | MIT (ours) | in-process | in-process, passes B2/B3 as-is | **Recommended.** |
| B. Yrs (`y-crdt`, Rust) via a UniFFI xcframework (`yswift`) | MIT | in-process | needs a `.binaryTarget`/xcframework, which **fails B2 today** | The best off-the-shelf option if Q1 is approved; Yjs-compatible wire format. |
| C. Automerge (Rust) via `automerge-swift` | MIT | in-process | binary target, which fails B2 | Heavier document model (JSON CRDT); text performance is fine since 2.x. |
| D. diamond-types (Rust) | permissive (verify the exact licence when pinning) | in-process | binary target, which fails B2 | The fastest text CRDT, but text-only and less mature as a library. |
| E. Rust CRDT in a helper process (like `flashtex-edit-ledger`) | MIT | an IPC hop per keystroke (the controller route measures 11–16 ms edit → durable) | impossible: no processes on iOS | Rejected. |

Why A rather than B: it keeps the iPad free of Rust and binary targets, so check B
needs no exception. It gives one implementation for both Apple clients (shared
through the symlink pattern that `apps/ios/Packages/FlashTeXPadKit/Package.swift`
already uses for `FlashTeXEditorCore`). It works natively in UTF-16 for the text
views, so no FFI copy is made per keystroke. The cost is owning a CRDT, about
2–3 kloc of Swift plus tests. This is mitigated by differential fuzzing against
the Rust oracle (§7.2). If the P0 spike misses its performance gate, or the owner
prefers not to own a CRDT, switch to B and amend B2 with an allow-list of
permissive xcframeworks. The `collab-v1` envelope does not change; only the op
encoding does.

Algorithm requirements for `FlashTeXCollabCore`:

- Items are **runs** `(OpId start, length, content, left-origin, right-origin)`,
  split on demand, so typing a sentence is one item, not 40.
- An order-statistics B-tree over runs keyed by visible length in **both** UTF-16
  units (for `NSTextView`/`UITextView` ranges) and UTF-8 bytes (for engine
  `edits` and runtime-v1 offsets). The unit is the Unicode scalar, as in
  collaboration-core, so no op can split a scalar. Grapheme clusters can still be
  split by concurrent edits; this is accepted, because editors render whatever
  converges.
- Fugue-style origin rules, to minimise interleaving when two people type at the
  same spot. The existing YATA tie-break (`OpId` order) is kept for determinism.
- `RelativePosition {id: OpId, assoc: before|after}` for carets, selections,
  folds, marks and undo, resolved in O(log n).
- A state vector (`[ReplicaId: counter]`) and "diff since vector", for sync.
- Bounded everything: max ops per message, max pending, max file size (8 MiB,
  matching `edit-ledger::MAX_DOCUMENT_BYTES`), and `CrdtError` semantics kept.

### 2.4 Composing with the editor, undo and the edit ledger

**Local edit path (Mac).** `shouldChangeTextIn` already captures `(range,
replacement)` into `lastEdit`. In a collaboration session, `commitUserChange` also
calls `collab.localEdit(path, utf16Range, replacement)`, which produces ops
(appended to the outbox, sent asynchronously) and an undo item. Everything after
that is unchanged: `parent.text = s` → `updateActiveText` → `EngineV3Session` and
`controllerSubmitEdit`. IME: nothing is emitted while `hasMarkedText()`. This is the
existing rule, so composition steps never reach peers.

**Remote edit path (Mac).** This is new. Remote ops resolve to UTF-16 ranges and are
applied in one `textStorage.beginEditing()`/`endEditing()` block with
`programmaticChanges += 1` and `undoManager.disableUndoRegistration()`. Ranges are
applied last-first, as `PendingEdit.groupedEdits` already does. The selection is
restored from its `RelativePosition`, not shifted by hand. Side state that is
"kept aligned with edits by `shouldChangeTextIn`" (`pendingClosers`, fold ranges via
`noteFoldEdit`, `marks.noteEdit`, `linkedSession`) must be shifted the same way.
The plan is to route remote ranges through the same helpers (`AutoClose.shifted`,
`noteFoldEdit`) so that there is one adjustment path. The `ShellModel` text is
then updated without echoing ops: a `remote` flag stops `commitUserChange` and
`bridgeTextChanged` from forwarding it. While the local user is composing
(`hasMarkedText`), remote ops are integrated into the CRDT immediately, but view
application is deferred until the composition commits (bounded: if a composition
outlasts 2 s, ops outside the marked range are applied and the marked range is
shifted). The `tv.string = text` reset in `updateNSView` must never run for a
remote change.

**Remote edit path (iPad).** It is the same, in `EditorController` around
`EditorTextStorage.replaceCharacters`. `editSerial` already lets the controller
notice a change exactly once whether UIKit or a programmatic edit made it. That is
the hook for telling a local edit from a remote one.

**Undo: local-only.** In a session, `NSTextView.allowsUndo` is turned off for
typing, and the editor registers its own `NSUndoManager` actions (so ⌘Z, the Edit
menu, action names and VoiceOver keep working) that call
`collab.undo(groupId)` / `redo`. An undo item stores the `OpId` runs the user
inserted (undo removes those still visible) and the runs they deleted (undo
re-inserts that content anchored at the tombstones, as new ops, as Yjs's
`UndoManager` does). Grouping keeps today's semantics: a capture insertion is one
step, a linked `\begin`/`\end` rename is one group (`openLinkedUndo`), and typing
coalesces on the existing `breakUndoCoalescing` boundaries. Undo never touches a
collaborator's text. Undoing your own insert that someone else typed inside
removes only your characters.

**Edit ledger.** It is unchanged in role. On each Mac, the converged text of each
open document is fed through `controllerSubmitEdit` / `replace_document` (expected
revision + SHA-256, one in flight, newest coalesced), exactly as today. This is the
composition that `collaboration-core`'s header already prescribes ("a single owner
replays the converged text through the edit ledger as one ordinary edit"). Two
consequences:

- The ledger's **durable undo/redo** (`history.rs`) reverts the last durable group
  regardless of author, which is wrong in a session. It must be disabled while
  collaborating, or replaced by "restore this revision", broadcast as ordinary
  edits by the restoring user (Q6). `EditHistoryPanel.swift` would then show
  author-attributed groups.
- Capture insertions (transfer-v1 `PreparedEdit`, bridge receipts) remain a local
  transaction of the Mac user who approves them. They become ordinary CRDT ops
  from that user, so bridge receipt semantics are unaffected.

**CRDT persistence.** Per session, in `~/Library/Application Support/FlashTeX/Collab/<session-id>/`
(on the iPad, the app container): a `Checkpoint`-style snapshot plus an append-only
op log tail, fsync'd in batches of ≤ 100 ms, and a durable **outbox** of local ops
not yet acknowledged by the hub. This is not stored under the project's `.flashtex/`,
because that directory may be synced or committed by the user.

### 2.5 Engine incremental compile

Remote edits reach the engine the same way local ones do. `EngineV3Session.compile`
diffs `sentTexts[path]` against the document text (`EngineV3Edits.splice`) and sends
`COMPILE.edits [{path, offset, delete, insert}]` (display-list-v3 §6.3). Nothing in
the protocol changes for edits. Two policies change:

- **Coalesce remote-only changes.** Every keystroke by anyone would otherwise
  supersede the running compile on every Mac. With five typists and hyperref
  (DESIGN.md §5.3: convergence rarely succeeds there, and every miss re-typesets to
  the end in the background), that thrashes. Local edits stay immediate. A batch
  of remote-only edits triggers a compile at most every 150 ms, and immediately
  if the edit falls inside the pages in the local viewport (L4, §5.4).
- **Multiple restart points.** The L3 restart is from the checkpoint before the
  *earliest* edited position in the batch. Edits in two chapters therefore cost
  a restart from the earlier one. The convergence-rate work in P4-FINISH directly
  determines how well collaboration scales, and should be measured with a
  multi-typist workload (§7.3).

### 2.6 Multi-file projects, binary assets, renames

The project is a map CRDT keyed by a stable **`FileId`** (128-bit random), not by
path:

```
FileEntry {
  path:    LWW<ProjectPath>   // (lamport, replica) wins; normalised by project-files' ProjectPath rules
  kind:    text | blob
  content: TextCRDT          // kind = text
         | LWW<BlobRef>      // kind = blob: {sha256, bytes, media_type}
  deleted: LWW<Bool>         // tombstone; content retained for restore
}
```

- **Renames** set `path`. Content history is keyed by `FileId`, so a rename that is
  concurrent with edits keeps both. A directory rename is a batch of path writes
  in one message. A file created concurrently in the old directory stays under
  the old name and is flagged.
- **Path collisions** (two `FileId`s with the same path after concurrent create or
  rename): the lowest `(lamport, replica)` keeps the path; the other materialises
  as `name (conflict <user>).ext` and raises a Problems-panel diagnostic. Nothing
  is silently dropped.
- **Delete vs concurrent edit:** delete wins, the content is kept in the tombstone,
  and the editor offers "Restore". This matches what users expect from Overleaf.
- **Blobs** (figures, PDFs, fonts) are content-addressed by SHA-256 (the digest
  `project-files` already computes). They are fetched on demand by hash in
  256 KiB chunks, verified, and stored in a local content-addressed store. Caps:
  50 MiB per blob and 500 MiB per session by default. A changed figure is a new
  `BlobRef` (LWW). `PasteImage.swift`'s write-then-`projectFilesChanged` flow
  becomes "hash, put blob, set entry".
- **Never synced:** compile outputs (`.aux`, `.log`, `.toc`, `.pdf`, `.synctex`,
  anything under `output_dir`), `.flashtex/`, `.git/`, `.DS_Store`, and files over
  the caps. `.bbl` is regenerated locally by each Mac's external tools (3.2
  `external_tools`), so it is not synced either.
- **Materialisation.** The hub writes the converged tree into the real project
  directory, through `project-files`' atomic save, so the user's folder is always
  current. Guest Macs materialise into the session directory (Q7). The engine
  compiles a project copy in any case (`EngineV3Session`'s `project.root`), and
  `projectFilesChanged(model:paths:)` already links newly arrived non-open files
  into it.
- **External edits on the hub** (another editor, `git pull`): today
  `DocumentWatcher` surfaces a conflict. In a session, a hub-side external change
  is diffed against the materialised text and ingested as edits by the hub user.
  The CRDT is the source of truth, and the file is its materialisation.

---

## 3. Transport and topology

### 3.1 Options

| | LAN star via hub (nearby link) | Internet relay | iCloud / CloudKit (`CKShare`) |
|---|---|---|---|
| Latency | ~5–20 ms on Wi-Fi | ~30–150 ms | Seconds; push is best-effort and rate-limited |
| Works across networks | No | Yes | Yes |
| Infrastructure | None | A server to run and pay for | Apple's, free at this scale |
| Identity | Invite QR | Accounts or invite tokens | Apple ID |
| Blocked by client-isolated Wi-Fi (eduroam, hotels) | **Yes** | No | No |
| Non-Apple future (DESIGN.md §16) | Yes | Yes | No |

**Recommendation.**

- **Phase 1–3: LAN star through the hub Mac.** This reuses the nearby link's
  machinery: Bonjour advertisement and discovery, the `NearbyListener` structure
  on `NWListener` with its caps (connections, per-session unacknowledged bytes,
  line and read caps, hello timeout), the reference client
  `apps/mac/tools/nearby-client` (as the base of a headless `collab-client`
  for tests), and the iPad's `MacLink`/`NearbySession` plumbing. It is a
  separate service type, `_flashtex-collab._tcp`, with a separate protocol
  (`collab-v1`): a capture pairing must never grant edit rights, and nearby-v1
  is JSON-lines while collaboration needs binary frames for display-list relay
  (§5.3).
- **Phase 4: an internet relay.** It is a stateless WebSocket/TLS 1.3 service
  (MIT, Rust or Swift-on-Linux, never linking the engine). It routes encrypted
  envelopes by session ID and keeps an encrypted, bounded store-and-forward log so
  that an offline guest can catch up without the hub. Because the CRDT needs no
  sequencer, the relay never decrypts. It is needed earlier than one would like,
  because campus and hotel Wi-Fi often isolate clients; this is the main reason
  LAN-only is just an MVP.
- **CloudKit: not for real-time sync.** It may be used later for encrypted
  snapshot backup between sessions (Q11).

### 3.2 Auth and identity

- **Invite.** The hub shows a QR code (and a copyable link):
  `flashtex-collab://join?v=1&s=<session-id>&k=<32 random bytes b64url>&fp=<hub cert SPKI sha256>&name=<project>`.
  It does **not** use a 6-digit code: nearby-v1 §5 itself flags the 20-bit code as
  its weakest point, and here the joiner is another person, not the user's own
  iPad. `k` is a single-use invite secret, which expires after 10 minutes or on
  use; Q2 covers multi-use invites.
- **Join.** The guest opens TLS to the hub and sends `join {invite_proof =
  HMAC(k, nonce‖guest_pubkey), display_name, device_kind}`. The hub user sees "Bob
  (iPad) wants to join", approves, and picks a role (`edit`/`view`). The hub then
  issues a per-participant `participant_id` (this is the CRDT `ReplicaId` space;
  see below) and a long-term token for reconnects, stored in the Keychain (the
  pattern of `KeychainPairStore`).
- **ReplicaId allocation.** IDs must be unique per session. The hub assigns them,
  with random 64-bit values and collision checking. A device that loses its state
  rejoins with a *new* ReplicaId, never reusing old counters; this is the
  `id_reuse_divergence.rs` lesson.
- **Display names are self-asserted** on the LAN and shown as such. Verified
  identity (for example Sign in with Apple) is only meaningful with the relay
  (Q2).
- **Revocation.** The hub removes a participant: their token is invalidated,
  their connection closed, and their CRDT history kept.

### 3.3 Encryption

- **LAN.** TLS 1.3 through Network.framework, with the hub's self-signed
  certificate pinned by the SPKI hash carried in the invite
  (`sec_protocol_options_set_verify_block`). Deviation from nearby-v1: nearby-v1
  uses TLS 1.2 PSK-only, which has **no forward secrecy** (its §5). That is
  acceptable for a user's own iPad but not for sessions among different people.
  ECDHE in TLS 1.3 gives forward secrecy at no extra cost (Q8).
- **Relay.** End to end: every envelope's payload is encrypted with
  ChaChaPoly (CryptoKit) under a session key distributed in the join handshake
  and rotated when a participant is removed. The relay sees session ID, sizes and
  timing only.

### 3.4 Offline and reconnect

- Every client keeps working offline: local ops go to the durable outbox, and the
  UI shows "Offline. Your edits will merge when <hub> is back."
- On reconnect: `sync_request {state_vector}` → `sync_reply {missing ops}` in both
  directions, then the outbox is flushed. This is idempotent, because duplicate
  delivery is `ApplyOutcome::Duplicate`. Out-of-order arrivals are buffered with
  `PendingOps` semantics, bounded, and an overflow triggers a full resync.
- **Hub restart** reloads the checkpoint and op log, and guests reconnect with
  backoff (0.5 s → 30 s). **Hub gone for good:** guests keep their state and can
  "Save a copy". Hub migration (another Mac becomes hub) is cheap because of the
  CRDT, but it is Phase 4.
- **Tombstone compaction** is only safe below the version every participant has
  acknowledged. The hub computes that floor; a participant absent beyond the
  floor rejoins from a snapshot, and its unsynced outbox is rebased as a text
  diff against the snapshot. This is Phase 4, and until then logs grow, bounded
  by the per-file caps.

### 3.5 `collab-v1` messages (sketch)

The framing is length-prefixed binary frames (type byte + length + payload), like
display-list-v3's framing, over the TLS stream. Control payloads are JSON; ops use
a compact binary encoding (varint `OpId`s, runs).

| Type | Direction | Payload |
|---|---|---|
| `join` / `join_ack` | guest ↔ hub | proof, name, device; `participant_id`, role, token, session pins (§5.4), the hub's environment digest |
| `sync_request` / `sync_reply` | both | state vector / ops since it (per file) plus the file-map ops |
| `update` | both | batch of ops (≤ 64 KiB), from the author's outbox, acknowledged with `ack {through}` |
| `awareness` | both (hub fans out) | presence (§4), ephemeral, last-writer-wins by `seq` |
| `blob_want` / `blob_chunk` | both | sha256 + range / bytes |
| `preview_subscribe` / `preview_frame` | iPad ↔ a Mac | display-list-v3 frames relayed verbatim (§5.3) |
| `compile_report` | Mac → all | `{state_vector_digest, read_set_digest, page_hashes[]}` (§5.4) |
| `leave` / `error` | both | as nearby-v1: unknown types answered, not fatal |

---

## 4. Presence

- **Data** (per participant, ephemeral, never in the CRDT or on disk):
  `{participant_id, name, colour_index, file_id, anchor: RelativePosition, head:
  RelativePosition, preview_page, following: participant_id?, seq}`. It is sent on
  change, coalesced to ≤ 20 Hz, and expires 10 s after the last heartbeat.
- **Colours** are a fixed 8-entry palette, assigned by the hub in join order, with
  light and dark values checked for contrast against both editor themes
  (`EditorThemes.swift`) and for deuteranopia separation. The colour is never the
  only cue: there is a name flag on the caret and the name appears in the
  participants list.
- **Mac rendering.** Remote selections are painted with layout-manager
  **temporary attributes** (as `MarkPainter` does for diagnostics, so the text
  storage and undo are untouched), only over the visible window. Remote carets
  are drawn by a lightweight overlay view with a name flag that appears on
  movement and fades after 1.5 s.
- **iPad rendering.** iOS's layout manager has no temporary attributes (as
  `EditorTextStorage.swift` notes), so selections and carets are an overlay layer
  positioned from the text layout's rects, recomputed on scroll and edit for the
  visible range only.
- **Elsewhere:** file tabs and the sidebar tree show avatars of the people in
  each file. The preview shows a thin coloured bar in the margin at each
  collaborator's caret, mapped through the source map (`EngineV3SourceMap.swift`,
  span IDs).
- **Follow mode.** Click a participant to follow them: switch to their file, keep
  their caret visible, and optionally keep their preview page. It reuses
  `CaretFollow.swift`'s re-arm semantics and stops on any local scroll, click or
  edit. A "Bring everyone here" presenter mode is a later addition.
- **Accessibility.** Joins and leaves are announced once each. Remote edits are
  never announced character by character; the existing per-run-loop coalescing is
  extended so that a remote edit near the caret posts at most one "Bob edited
  line 42" every 5 s. Each remote caret is reachable through the editor rotor
  (`EditorRotor.swift`) as "Bob's cursor".

---

## 5. Compile model

### 5.1 Options

- **(a) Every client compiles locally from the converged state.** The latency is
  the local §1.2 targets after the text arrives; there is no single point of
  failure and no new server.
- **(b) A shared host compiles and streams pages to everyone.** It gives one truth
  for output, but it costs a network hop on every preview, the host's CPU scales
  with participants, and the host becomes a single point of failure.

### 5.2 Recommendation: (a) on Macs, plus a relayed preview for iPads

- Every Mac runs its own `flashtex-host` against its materialised copy, as today.
- An iPad cannot compile. iOS cannot spawn the host process, and linking the GPL
  engine into the MIT iPad app is barred by DESIGN.md §3 and by check B. The iPad
  therefore subscribes to a **preview source**: by default the hub, or the Mac of
  the person it is following. That Mac forwards its display-list-v3 frames
  (`PAGES`, page frames, `FONT`, images, `SOURCES`, `DIAG`) verbatim in
  `preview_frame`. The relay is data, not code: display-list-v3 is an MIT
  specification, and `FlashTeXDisplayListV3`/`FlashTeXPreviewV3` import only
  Foundation, CoreGraphics, CoreText, CoreImage, ImageIO and IOSurface, all of
  which exist on iOS. They would be added to `FlashTeXPadKit` through the symlink
  pattern. Font programs are sent once per session (`have_fonts`).
- **iPad risks.** Type 1 through `CGFont(CGDataProvider)` is verified on macOS 26
  only (DESIGN.md §6.2 calls it undocumented). It must be canaried on iPadOS. The
  fallback is the Mac sending rasterised tiles at the iPad's scale.

### 5.3 Determinism

TeX is deterministic given identical inputs, and DESIGN.md §4.5 already pins what
is not an input in a single session. Across machines, "identical inputs" means:

| Input | Same on every peer? | Mechanism |
|---|---|---|
| Project sources and blobs | Yes, by convergence | CRDT plus content-addressed blobs |
| `\time`, `\day`, `\month`, `\year`, random seed | Only if pinned **per session**, not per editing session per machine | The hub chooses `source_date_epoch` and the seed at session start and sends them in `join_ack`. This needs an additive `COMPILE` key (display-list-v3), Q3. |
| Main file, job name, format name, `shell_escape`, `external_tools` | Yes | session pins in `join_ack` |
| Engine build and format | Not necessarily | environment digest exchanged; mismatch → warning |
| TeX tree (`latex.ltx`, packages, `pdftex.map`, fonts); DESIGN.md §4.4 **prefers the user's TeX Live** | **Often not** (TL 2025 vs 2026, `TEXMFHOME` differences) | environment digest exchanged; mismatch policy is Q5 |
| `.aux`/`.toc` from previous runs | Equal at the fixpoint | multi-pass to fixpoint (L5, 3.2 `external_tools`) |
| `\write18` output, `\pdffilemoddate`, `\input` of files outside the project | No | barred in sessions (§6.2) |

The guarantee offered is: same converged state, same pins and same read set ⇒ the
same pages, after the aux fixpoint. It is **checked, not assumed**. After a compile
reaches the fixpoint, each Mac sends `compile_report {state_vector_digest,
read_set_digest, page_hashes}`, using display-list-v3's per-page content hashes and
the S₀ read-set key of §5.1. Peers that compiled the same state compare. A mismatch
shows "Your preview differs from Alice's on pages 3–4 (TeX Live 2025 vs 2026)" in
the Problems panel. This turns a support mystery into a diagnosable state.

---

## 6. Licensing and security

### 6.1 Licensing

- The GPL engine stays a separate process (`flashtex-host`), reached only over
  display-list-v3. Collaboration adds **no** link-time edge to it. Each Mac runs
  its own host, and iPads receive display-list *data*.
- `FlashTeXCollabCore` (Swift) and the evolved `crates/collaboration-core` (Rust,
  reference and oracle) are MIT and have no dependencies. The relay, if built, is
  MIT and must not depend on `flashtex-engine`; check A covers it if it lives in a
  cargo workspace.
- Option B or C (Yrs/Automerge, MIT) would be licence-compatible, but they fail
  check **B2** (binary target or xcframework under `apps/ios`). That is a policy
  rule, not a licence issue, so adopting them needs the owner to amend the rule
  (Q1).
- **Relaying font programs** (from the user's TeX Live: LPPL, OFL, GPL with font
  exception, and so on) to another person's device inside display-list frames is
  comparable to sending them a PDF with embedded subsets. It still belongs in the
  §3 legal review the owner is arranging (Q10).
- Network use of the GPL engine on a future shared compile host is not triggered:
  GPL-2/3 has no network clause (unlike AGPL), and xpdf's GPL v2/v3-only terms
  (DESIGN.md §3) are unaffected.

### 6.2 Security (collaborators are untrusted input)

Joining a session means compiling **another person's** source on your Mac. This is
the auto-compile-of-untrusted-projects concern (DESIGN.md §4.5, O9) at keystroke
frequency, plus a new **exfiltration channel**. If a guest writes
`\input{/Users/alice/.ssh/id_rsa}` and the hub is the iPad guest's preview source,
the file's text appears in the relayed preview. So in any session:

- Read confinement is mandatory: project files plus TeX trees only. This
  overrides TeX Live's `openin_any=a` (the open item in §4.5); `openout` is
  confined to the output directory.
- `shell_escape` is pinned to `off` for the session, or `restricted` if the owner
  decides so under O9. The per-project "full shell escape" opt-in is unavailable
  while collaborating.
- Diagnostics and logs relayed to other participants are scrubbed of absolute
  paths outside the project, as are `DIAG` frames.
- Caps on everything a peer can send: op batch size, pending ops, blob sizes,
  presence rate, and connections per participant. These mirror the nearby-v1
  cap table.

---

## 7. Phased plan, acceptance criteria and test strategy

### 7.1 Phases

| Phase | Scope | Exit gate (all measured on M1 Max / iPad simulator unless stated) |
|---|---|---|
| **P0: spike** (≈ 2 wk) | `FlashTeXCollabCore`: runs, B-tree, UTF-16/UTF-8 indices, `RelativePosition`, state vectors. `collaboration-core` gains runs and an op-log fixture exporter. | ≤ 1 ms p95 local insert and remote apply on a 1 MB file with 100 k tombstones; ≤ 150 B/char memory; 10⁶ fuzz cases, 0 divergences, Swift ≡ Rust on all fixtures. **Decision point A vs B.** |
| **P1: two Macs, one file, LAN** | `collab-v1` hub/guest, QR invite with approval, local undo, remote edit path in `SourceEditorView`, carets and selections, durable outbox, reconnect. | Convergence integration tests green; remote keystroke → remote text commit ≤ 50 ms p95 on LAN; local keystroke overhead ≤ 1 ms p95; the §7.3 editor integration suite green; the ledger receives converged text with no revision conflicts in a 30-min soak. |
| **P2: projects and iPad text** | File map CRDT, renames, collisions, blobs, materialisation, hub external-change ingest; iPad multi-file editor bound to the CRDT; follow mode; participants list. | Mac–iPad interop test (§7.4) green in CI; a 5-peer headless soak (30 min, 8 chars/s each, 300-page thesis) converges, with logs bounded by caps. |
| **P3: preview for everyone** | iPad `preview_subscribe`/`preview_frame` with `FlashTeXPreviewV3` on iPadOS; session pins (Q3); `compile_report` determinism check; remote-edit compile coalescing (§2.5). | iPad preview within ≤ 50 ms p95 of the source Mac's commit on LAN; Type 1 canary on iPadOS; two Macs with identical environments report identical page hashes on the 83 parity fixtures under concurrent editing; a deliberate TL mismatch is diagnosed. |
| **P4: beyond the LAN** | Relay with E2EE and store-and-forward; hub migration; tombstone compaction; verified identity (if Q2). | Remote keystroke ≤ 150 ms p95 same-continent; relay holds no plaintext (test inspects stored bytes); compaction keeps a 30-day session's log ≤ 2× the text size. |

### 7.2 Convergence testing

- **Property and fuzz tests (both implementations).** N ∈ {2..5} replicas run
  random ops: insert and delete runs including multi-byte scalars (CJK, emoji,
  combining marks), undo and redo, renames, deletes, path collisions, and
  blob-ref writes. Delivery is random: reorder, duplicate, drop then redeliver,
  and partition then heal. The asserted invariants are: (1) all replicas have
  identical text, file map and structure digest after full delivery; (2) with
  causal, non-concurrent delivery the text equals a sequential reference string;
  (3) no scalar is ever split; (4) every `RelativePosition` resolves; (5) undo
  never removes another replica's characters. Rust uses `cargo test` plus
  `cargo-fuzz`; Swift uses XCTest with seeded RNG, and the failing seed is printed.
- **Differential.** Rust exports op logs as fixtures
  (`crates/collaboration-core/tests/fixtures/collab-v1/*.json`); Swift replays them
  and compares digests, and vice versa. The existing `adversarial.rs`,
  `interrupted_delivery.rs` and `id_reuse_divergence.rs` cases are the first
  fixtures. This is the guard against the two implementations drifting.
- **Real network.** The headless `collab-client` (grown from
  `apps/mac/tools/nearby-client`) runs 3–5 scripted peers against a hub with
  injected latency, loss and disconnects.

### 7.3 Editor integration tests (Mac, then iPad)

These cover remote edits arriving: during IME composition, with the completion
popup open, during a linked `\begin`/`\end` rename, with pending auto-closers
(`pendingClosers`), inside a folded region, over diagnostic marks, at the caret,
and inside the local selection. Undo and redo after interleaved remote edits must
restore exactly the local text. VoiceOver posts no per-character announcements.
`tv.string = text` is never hit for a remote change, which is asserted with a
counter. Engine: concurrent remote edits produce the same pages as a full compile
of the converged text, reusing the parity harness. A multi-typist workload (2–5
synthetic typists in different chapters) is added to the T7 convergence-rate
measurement.

### 7.4 Mac–iPad interop test

The base is the existing simulator harness (nearby-v1 §9). The Mac app hosts in
CI; the iPad app runs in the booted simulator under XCUITest; a third headless
`collab-client` peer is added. A scripted concurrent typing sequence runs on all
three, including the same-spot interleaving case and an iPad disconnect and
reconnect mid-burst. Asserted: identical text on all three (exposed through an
accessibility value carrying the SHA-256 of the buffer); the remote caret is
visible in each editor (accessibility element "<name>'s cursor"); in P3, the iPad
received and drew the preview page whose hash matches the Mac's `compile_report`.

### 7.5 Latency targets (summary)

| Path | Target |
|---|---|
| Local keystroke overhead from collaboration (1 MB file) | ≤ 1 ms p95 |
| Remote keystroke → remote editor commit (LAN / relay) | ≤ 50 ms / ≤ 150 ms p95 |
| Presence update visible | ≤ 100 ms p95 |
| Remote edit → each Mac's preview | remote-text latency + §1.2 targets (+ ≤ 150 ms remote coalescing when outside the viewport) |
| Source Mac preview commit → iPad commit (LAN) | ≤ 50 ms p95 |
| Join: 1 MB of text ready to edit | ≤ 2 s; blobs stream afterwards |

---

## 8. Open questions and risks

### 8.1 Open questions for the owner and the Commander

- **Q1 (owner).** Own a Swift CRDT (recommended), or amend licence check B2 to
  allow allow-listed permissive xcframeworks under `apps/ios` so that Yrs or
  Automerge can be used?
- **Q2 (owner).** Is LAN-only acceptable for the MVP? For the relay: who hosts it,
  what budget, and is identity invite-only or account-based (Sign in with Apple)?
  Should invites be single-use or multi-use links?
- **Q3 (Commander, engine lane).** Add an additive display-list-v3 `COMPILE` key for
  session pins (`source_date_epoch`, random seed) so that every peer's `\today`
  and pgf randomness agree.
- **Q4 (owner).** Security defaults for compiling collaborators' content: mandatory
  read confinement and `shell_escape: off` in sessions (§6.2). This interacts with
  O9 and the open `openin_any` item in §4.5.
- **Q5 (owner).** TeX tree mismatch policy: warn only (recommended for the MVP), or
  require a session-pinned content-addressed bundle (the §4.4 fallback) so that
  outputs are identical by construction?
- **Q6 (Commander).** The edit ledger's durable undo/history in sessions: disable
  it, or turn it into an author-attributed "restore revision"?
- **Q7 (owner).** Where do guests' working copies live, and may a guest save or
  export the project to their own disk?
- **Q8 (Commander).** Transport: TLS 1.3 with a pinned certificate (forward
  secrecy, recommended), or reuse nearby-v1's TLS 1.2 PSK verbatim (less new
  code, no forward secrecy)?
- **Q9 (Commander).** Ownership: a new lane for `FlashTeXCollabCore` plus
  `collab-v1`. Because the target would be symlinked into `FlashTeXPadKit`, every
  change runs both the Mac and iPad test suites, as with `FlashTeXEditorCore`.
- **Q10 (owner).** Add relaying of font programs and pages to other people's
  devices to the §3 legal review.
- **Q11 (owner).** CloudKit: none, or encrypted snapshot backup between sessions?
- **Q12 (Commander).** Should collaboration get its own DESIGN.md section and
  phase gates (§9), or stay a separate design under `docs/design/live-collab/`?

### 8.2 Risks

| Risk | Likelihood / impact | Mitigation |
|---|---|---|
| Editor integration (NSTextView undo, IME, auto-close, linked environments, folds) with remote edits: the most intricate part | High / high | A single ranged remote path through the existing shift helpers; the §7.3 suite before P1 exit; IME deferral rule |
| Owning a CRDT: subtle convergence bugs | Medium / high | Differential fuzzing against the Rust oracle; adversarial fixtures carried over; switch to option B at decision point A |
| Incremental compile thrash with several typists (low convergence on hyperref, background re-typeset per keystroke × N) | High / medium | Remote coalescing and viewport priority (§2.5); measure in T7; benefits from P4-FINISH |
| Divergent previews from different TeX Live installs | High / medium | Environment digest plus `compile_report` diagnosis; Q5 bundle option |
| Exfiltration or code execution via collaborators' source | Medium / high | §6.2 confinement, `shell_escape` off, path scrubbing |
| Client-isolated Wi-Fi blocks LAN sessions | High on campuses / medium | Clear "can't reach hub" diagnostics; prioritise P4 relay if the owner's users are students |
| Tombstone and log growth in long sessions | Medium / low (MVP) | RLE; per-file caps; P4 compaction |
| Type 1 fonts on iPadOS via `CGFont` | Unknown / medium | Canary in P3; tile fallback |
| Two implementations (Swift product, Rust oracle) drift | Medium / medium | Shared fixtures in CI on both sides; the contract doc is normative |
| Local Network permission prompts (macOS, iPadOS) confuse users | Medium / low | `NSLocalNetworkUsageDescription`, `NSBonjourServices` (nearby-v1 §6 already lists this for `_flashtex._tcp`) |

---

## 9. If adopted: what changes where

- **DESIGN.md (Commander):** a short section or a pointer here, the Q3 protocol
  key, the Q4 security default, and phase gates P0–P4 (Q12).
- **New:** `apps/mac/Sources/FlashTeXCollabCore` (platform-free, symlinked into
  `apps/ios/Packages/FlashTeXPadKit`); `docs/contracts/collab-v1.md`;
  `apps/mac/tools/collab-client`.
- **Changed:** `crates/collaboration-core` (runs, fixture export; still no deps);
  `SourceEditorView.swift` (remote path, custom undo registration);
  `ShellModel.updateActiveText` (remote flag); `EngineV3Session` (remote
  coalescing, session pins); `EditHistoryPanel.swift` (attribution); the iPad
  `EditorController`/`EditorTextStorage`/`PadDocument` (multi-file, CRDT binding,
  preview view).
- **Unchanged:** `flashtex-engine`, the licence boundary checks (unless Q1),
  transfer-v1 and nearby-v1 (captures keep working alongside sessions), and the
  edit ledger's wire format.

---

## 10. Status

**Adopted 2026-10-04.** The owner asked to start building Live Share; the Commander ruled that this
proposal's recommendations and its defaults for Q1–Q11 apply. Q12 stays open: this remains a
separate design under `docs/design/live-collab/`.

### 10.1 P0 (lane LIVE-SHARE-P0, PR #1488)

Built: `apps/mac/Sources/FlashTeXCollabCore` (option A of §2.3, pure Swift, Foundation only,
symlinked into `FlashTeXPadKit`; no app links it yet) and the Rust oracle
`crates/collaboration-core/src/v1`. Both cover the text CRDT (FugueMax, runs, B+tree indexed by
scalars, UTF-16 and UTF-8), `RelativePosition`, state vectors and diffs, the file-map CRDT keyed by
`FileId`, and the `collab-v1` codec. The Swift side also has local undo and redo in CRDT ids. The
normative contract is [`docs/contracts/collab-v1.md`](../../contracts/collab-v1.md). No UI,
networking or app integration is in P0.

Gate, measured on mac-m1max-a (M1 Max), `PerformanceGateTests` in release. The document is 1 MiB,
1,048,576 visible scalars, with 190,075 tombstones and 196,201 runs, typed one operation per
keystroke. The first column is a run at normal load; the second ran while other sessions held the
load average near 200.

| Gate (§7.1) | Target | Measured | Under load |
|---|---|---|---|
| Local insert p95 | ≤ 1 ms | 9.4 µs (p99 19.5 µs) | 6.5 µs |
| Remote apply p95 | ≤ 1 ms | 1.6 µs (p99 4.2 µs) | 2.4 µs |
| Memory per character (footprint delta, op log and indexes included) | ≤ 150 B | 61.0 B | 61.2 B |
| Fuzz cases, divergences | 10⁶, 0 | 10⁶ Swift and 10⁶ Rust, 0 | — |
| Swift ≡ Rust on all fixtures | yes | 18 fixtures (14 recorded by the oracle, 4 by Swift with undo), each in 3 delivery orders, identical in both | — |

Also measured:

- Local delete p95 9.8 µs. Throughput: 257k local keystroke ops/s while typing the document, and
  789k remote ops/s.
- A 10k-op divergence on each side merges in 61 ms in total, codec included.
- A full join of the 1 MiB document (191k ops, a 4.7 MiB frame) takes 1.28 s against the ≤ 2 s
  target of §7.5.
- Single outliers of 10–100 ms track machine load. They also hit read-only calls, so they are not
  algorithmic.

**Decision point A vs B: A.** The gate passes by two orders of magnitude, so nothing argues for
amending B2 to take Yrs.

### 10.2 Next: P1 (two Macs, one file, LAN)

1. `collab-v1` transport: a hub listener on `_flashtex-collab._tcp` with nearby-v1's caps, TLS 1.3
   with a pinned certificate (Q8), the QR invite with approval, and a durable outbox with
   `update`/`ack` and reconnect via `sync_request`. **Replica binding** (required by
   collab-v1 §2.1, from the #1488 review): the session layer binds each connection to the replica ids
   the hub assigned it and drops, before the CRDT, any operation whose ids name another replica, so
   a peer cannot forge another's ids (an id conflict is refused, but whichever version arrives first
   wins). Guests accept relayed operations only from the hub's connection. A peer whose operations
   hit `pending full` (count or the 16 MiB byte cap) is resynchronised.
2. The remote edit path in `SourceEditorView`: ranged application, the shift helpers, IME deferral,
   and no `tv.string` reset. Custom `NSUndoManager` registration onto `TextUndoManager`.
3. Carets and selections from `awareness`.
4. Converged text into the edit ledger, with its durable undo off in sessions (Q6).
5. The §7.3 editor integration suite, and the P1 latency gates.

### 10.3 P1 (lane LIVE-SHARE-P1): built, behind Settings ▸ Live Share (preview), off by default

Three stacked PRs: #1530 (transport and session layer, `FlashTeXCollabSession`, collab-v1 §7),
#1534 (editor integration), and the presence/UI PR. What is in:

- Transport as §3.1–§3.3: TLS 1.3 only, with the hub's in-memory P-256 identity pinned by its SPKI
  SHA-256; single-use 10-minute invites with host approval (edit, view or deny); token reconnect with
  backoff and two-way state-vector resync; replica binding; presence fan-out.
- Editor as §2.4: local edits are taken from the text storage one at a time (IME steps are held back
  until commit). Remote operations are applied as minimal storage edits, never `tv.string`, with
  marks, closers and folds shifted, the selection restored by relative position and the viewport kept.
  Remote operations wait while the local user composes. ⌘Z is local-only and coalesced through
  `TextUndoManager`. The edit ledger's durable undo is refused during a session (Q6).
- Presence as §4: carets, selections and fading name flags drawn by an overlay over the visible text;
  participants in the status bar and the session panel.
- Compile as §5.2: each Mac compiles its own converged copy; a guest's copy is a normal project under
  Application Support/FlashTeX/Collab/<instance>/<session>.
- Two instances on one Mac: the invite names the host machine, and a guest on that machine connects
  over loopback (`FLASHTEX_INSTANCE` keeps the copies apart).

Measured on mac-m1max-a: a remote keystroke reaches the other editor in p50 6.2 ms and p95 ≤ 7.5 ms
over loopback TLS (n = 200). These are upper bounds that include up to 5 ms of test polling. LAN
latency was not measured. Tests: 14 session tests over loopback TLS, 7 hosted two-editor tests and 5
controller tests (host and guest `ShellModel`s). A two-process smoke run of the packaged app (host plus
guest instance, invite by file, debug-only auto-approve) converged to disk.

Deviations and follow-ups (not in P1):

- P1 shares every text source of the project (not one file), but no binary files. Figures and blob
  sync are P2.
- The outbox is in memory. An app crash loses unacknowledged edits; the durable outbox and the session
  op log of §2.4 are follow-ups.
- Session pins (§5.3) are sent in `join_ack` but not yet applied to compiles. The engine host already
  honours `SOURCE_DATE_EPOCH`/`FORCE_SOURCE_DATE`; applying them needs a host restart on join.
- The IME rule waits for the commit. The 2 s partial-apply bound of §2.4 is not implemented.
- Not done yet: accessibility of remote carets (the rotor entry, the "Bob edited line 42" coalescing),
  follow mode, and per-file avatars in tabs.
- New files, renames and deletes made during a session are not shared; the file map supports them.
- Not measured: the 30-minute ledger soak and LAN latency.

Security review of P1 (fixed before merge):

- A guest's file-map operations never integrate. The host writes only the files it shared, at their
  shared paths, never a dotfile, a path through a link, or a case or normalisation twin. A guest's copy
  obeys the same rules.
- Before join, frames are limited to 4 KiB. Per address there are at most 4 connections. Joiners waiting
  for approval count against the participant cap. Awareness is rate-limited at the hub, and the Bonjour
  TXT names no project.
- Session compiles (a guest's copy, or any project while hosting) run with `shell_escape` off and
  external tools off (§6.2), whatever the project's trust.
- Turning the setting off ends the session. The launch automation exists only in debug builds. A
  remote change that does not fit the buffer resynchronises it from the CRDT, and operations held for a
  composition are bounded (past the bound, the composition is committed).
- `flashtex.toml` (and every `.toml`) is never shared: the manifest decides package sources and
  fetching, fonts and the engine, and stays the host's own for the session.
- Read confinement (§6.2). Session compiles run in a host launched with `FLASHTEX_CONFINE_READS=1` and
  `FLASHTEX_CONFINE_ROOTS` (the real project folder). The gate sits at the engine's lowest layers, the
  resolver lookups (`resolve`, `resolve_ex`) and `open_input`'s output-directory shortcut
  (`input_path`, `find_input`), so every file the engine opens goes through it: `\input`, `\openin`,
  `\pdfobj file`, `\font` and its TFM/VF/encoding/font files, `\pdfmapfile`, `\pdfmapline`, images,
  and the file primitives. Two rules:
  - The name asked for may not be absolute, start with `~`, contain `$` or have a `..` component.
  - What was found, followed through every link, must lie in the project, the job's folder or the
    output folder, or be a link-free search-path hit (a TeX tree file). A relative link to a file
    outside the project is therefore refused.

  The format and the pool, which come from the host's own command line, are exempt. A document's
  tex.web device name (`TeXformats:/etc/hosts`) cannot reach that exemption: as in pdfTeX, such a name
  is literal, and pdfTeX answers "I can't find file `TeXformats:/etc/hosts'". This also fixed a
  parity bug, covered by lockstep case 2575, for both reading and writing. No mktex script
  runs (`MKTEXTFM`, `MKTEXPK`, `MKTEXMF` and `MKTEXTEX` are 0), so a `\font` name can never start
  METAFONT. kpathsea's `openin_any = p` is not enough: measured against TeX Live 2026's pdfTeX, it
  still reads `/etc/hosts`, `~/x` and `../x` through `\openin`.

  Writes: the same host runs with `openout_any = p` and no `TEXMFOUTPUT`. A relative `\openout` name
  lands in the project copy's output folder, never next to the sources, and absolute or `..` names
  are refused.

  Without the variables the engine is unchanged; the lockstep passes in full with them unset. The
  app relaunches the host when the confinement changes, the keystroke fast path never sends to a
  host whose confinement does not match, and leaving a session clears the copy's output folder
  (`.aux` and the like). Files a host's own `flashtex.toml` names outside the project (texinputs) are
  not readable during a session.
- Guest-written files are marked. Every host file that a guest's text reached gets a
  `com.apple.quarantine` mark (one event per session). The app's saves copy extended attributes, so the
  mark stays. The trust check (`EngineV3Trust`) counts a quarantined file that did not come with the
  project's own download, so after the session the project compiles untrusted (shell escape off, no
  external tools) until the user trusts it again. During the session, compiles are pinned anyway.
- A guest takes at most 200 files and 32 MiB. Writes go through `O_CREAT | O_EXCL | O_NOFOLLOW`
  temporary files that are renamed into place. IPv6 peers count against the per-address cap by
  their /64.
