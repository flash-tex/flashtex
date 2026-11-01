# collab-v1: live collaboration data model and wire format

Status: **P0** (lane LIVE-SHARE-P0, 2026-10-04). Design: [`docs/design/live-collab/PROPOSAL.md`](../design/live-collab/PROPOSAL.md).
This document is normative for both implementations:

| | Path | Role |
|---|---|---|
| Product | `apps/mac/Sources/FlashTeXCollabCore` (Swift, MIT, Foundation only; symlinked into `apps/ios/Packages/FlashTeXPadKit`) | runs in the Mac and iPad apps |
| Oracle | `crates/collaboration-core/src/v1` (Rust, MIT, no dependencies) | reference for differential testing |

Both replay the fixtures in `crates/collaboration-core/tests/fixtures/collab-v1/` (§6). A change to
anything below changes both implementations and the fixtures in one PR.

## 1. Conventions

- **varint**: unsigned LEB128, at most 10 bytes, value ≤ 2⁶⁴−1. An overlong encoding (a final `0x00`
  byte after the first) is an error.
- **bytes**: varint length, then that many bytes. **string**: bytes that are valid UTF-8.
- **id**: varint `replica`, varint `counter`. **opt-id**: `u8 0`, or `u8 1` then an id.
- **file-id**: 16 raw bytes (128 random bits).
- Text is a sequence of **Unicode scalars**. No operation addresses less than a scalar, so none can
  split one. (A grapheme can still be split by concurrent edits; editors render what converges.)
- Integers in frame headers are little-endian (as in display-list-v3).

## 2. Data model

### 2.1 Units, ids and state vectors

A project has documents: one **file map** and one **text document** per text file. In each document,
every replica numbers the *units* it creates `0, 1, 2, …` with no gaps: one unit per inserted scalar,
one per deleted scalar, one per file-map change. A unit's id is `(replica, counter)`. Replica ids are
random 64-bit values assigned by the hub; a replica that loses its state rejoins with a new id and
never reuses counters.

**Replica binding (session layer, P1).** Nothing in an operation proves who made it, so the data
model alone cannot stop a peer from forging another replica's ids (an id reused with different
content is refused, §2.4, but whichever version arrives first wins). The session layer must bind each
connection to the replica ids the hub assigned it and drop, before they reach the CRDT, operations
whose ids name any other replica. The hub relays others' operations verbatim, so guests accept them
only from the hub's connection.

Units of one replica apply in counter order. A document's **state vector** maps each replica to the
number of its units applied (the next counter expected). Missing replicas are 0.

### 2.2 Text operations

```
insert: u8 1, id, opt-id origin_left, opt-id origin_right, string content (≥ 1 scalar)
delete: u8 2, id, id target, varint len (≥ 1)
```

- **insert** of n scalars consumes counters `id … id+n−1`. Unit k (0-based) is the scalar
  `content[k]` with id `id+k`, left origin `origin_left` if k = 0 else `id+k−1`, and right origin
  `origin_right`. Origins are the ids of the scalars **raw-adjacent** (tombstones included) to the
  insertion point when the author made it; none means the start or the end. A run is therefore defined
  unit by unit, and any suffix of it is itself a valid insert (with left origin = the unit before it).
- **delete** of len consumes counters `id … id+len−1`; unit k tombstones the scalar `target+k`. The
  targets are consecutive ids of one replica. Deleting a tombstone again is a no-op.
- Deleted scalars are never removed (tombstones keep their content and position).

**Local insert at visible offset p** (the rule both implementations use): let `r` be the raw position
of visible scalar p (or the end); `origin_right` = the scalar at `r`, `origin_left` = the raw scalar
just before it (often a tombstone). **Local delete**: one delete per maximal run of consecutive
target ids, in document order.

### 2.3 Integration (FugueMax)

To integrate a unit `x` with origins `L`, `R` (raw indices `left` = index of L or −1, `right` = index
of R or the raw length), exactly Gentle's `integrateYjsMod` (which *The Art of the Fugue* shows
equivalent to FugueMax):

```
dest = left + 1; scanning = false; i = left + 1
loop:
  if !scanning: dest = i
  if i == raw length or i == right: break
  o = item i; oleft = index of o.origin_left (or −1); oright = index of o.origin_right (or raw length)
  if oleft < left: break
  if oleft == left:
    if oright < right: scanning = true
    elif oright == right:
      if x.replica < o.replica: break
      scanning = false
    else: scanning = false
  i += 1
insert x at dest
```

An implementation may integrate a run at once: the units after a run's first have their predecessor
as left origin, so the rule never stops inside the run, and only the first unit of each stored run
needs the rule (the others always satisfy `oleft > left`).

### 2.4 Applying an operation

Given an operation with id `(r, c)` and length n, and the document's state vector `sv`:

1. `c > sv[r]`: **missing dependency** `(r, sv[r])`; retry later.
2. The first `min(sv[r] − c, n)` units are already applied: each must match what is stored (insert:
   scalar, left origin, right origin; delete: target), else **id conflict** and the operation is
   refused. If all n were applied: **duplicate**, nothing changes. Otherwise continue with the suffix.
3. Every origin and target must name a stored scalar. If one names a unit already covered by `sv`
   that is not a scalar (a deletion unit): **malformed**. If one is not covered yet: **missing
   dependency**.
4. Insert: if both origins exist and `index(L) ≥ index(R)`: **malformed**.
5. If stored content (UTF-8 bytes, tombstones included) would exceed 8 MiB: **document full**.
6. Integrate (§2.3) or tombstone, then `sv[r] = c + n`.

Every rejection is deterministic given the same applied set, so all replicas reject alike (the
fixtures check the error kinds, §6). An id conflict is the exception: which of two versions of an id
is refused depends on which arrives first, which is why replica binding (§2.1) matters. A refused
operation blocks its replica's later units in that document; the session must resynchronise that
participant.

A project buffers retryable operations and retries each when the dependency it named arrives. The
buffer holds each distinct operation once (an exact redelivery is not parked again) and is bounded
by count (65 536) and by bytes (16 MiB, counting each operation as its variable payload, insert
content, path or media type, plus 64); an operation over either bound is refused as
**pending full**, and the session should resynchronise that peer instead.

### 2.5 Relative positions

`{anchor: id?, assoc: before|after}`. `after` sticks to the scalar on its left (no anchor: offset 0);
`before` sticks to the scalar on its right (no anchor: the end). Resolving an anchored position counts
the visible scalars before the anchor, plus one for `after` when the anchor is visible. A deleted
anchor resolves to where it was.

### 2.6 File map

```
file op: id, varint lamport, file-id, then one of
  u8 1 create:      u8 kind (0 text, 1 blob), string path
  u8 2 set_path:    string path
  u8 3 set_blob:    32 bytes sha256, varint bytes, string media_type (≤ 255 bytes)
  u8 4 set_deleted: u8 0|1
```

Each op consumes one counter. `lamport` ≥ 1; a replica's next op uses `max(lamport seen) + 1`. Fields
are last-writer-wins registers ordered by stamp `(lamport, replica)`; the kind is fixed at creation;
`create` sets `path` and `deleted = false` with its stamp. A set on an unknown file waits (**unknown
file**); a second create of the same file id is an **id conflict**; `set_blob` on a text file and an
invalid path are **malformed**. A valid path is 1–1024 bytes, `/`-separated, with no empty, `.` or
`..` segment, no `\` and no NUL. **The rule is over UTF-8 bytes**, never over grapheme clusters: a
combining mark after `/` or `\` forms one grapheme with it, so a grapheme-based check would accept
`../\u{301}etc/passwd` or `a\\\u{301}b` (the `hostile-paths` fixture has these). Comparisons are
byte-exact too (no Unicode normalisation; NFC and NFD names are different files).

**Materialisation**: live (not deleted) files are grouped by path; in each group the lowest path
stamp (then file id) keeps the path and every other is shown at
`dir/stem (conflict <replica as 16 hex digits>-<lamport>).ext`. Delete beats a concurrent edit (the
text keeps converging and is restorable). Text edits never touch the file map.

### 2.7 Undo (product only)

Undo is local and expressed in ids: a step records the spans its user inserted and the spans they
deleted. Undoing deletes the still-visible scalars of its own inserted spans (following any copy an
earlier undo or redo restored them as) and re-inserts a copy of each deleted piece, as new ordinary
operations, right after that piece's tombstone. Its inverse is the redo step. Undo never removes
another replica's scalars. The operations it emits are ordinary §2.2 operations; peers need nothing
special.

## 3. Framing

Every message is one frame: `u32 LE length` (kind + body, 1 … 2²⁴), `u8 kind`, body. A length of 0 or
over 2²⁴ is a corrupt stream. A binary body with bytes left over is an error.

## 4. Messages

| kind | name | body | direction |
|---|---|---|---|
| `0x01` | `join` | JSON `{invite_proof, nonce, guest_pubkey, display_name, device_kind, token?}` | guest → hub |
| `0x02` | `join_ack` | JSON `{participant_id, role, token, colour_index, pins: {main, job_name?, source_date_epoch, random_seed, shell_escape, external_tools, read_confinement}, environment_digest}` | hub → guest |
| `0x03` | `sync_request` | varint n, n × {doc, varint m, m × (varint replica, varint next)} | both |
| `0x04` | `sync_reply` | sections | both |
| `0x05` | `update` | varint seq, sections | both |
| `0x06` | `ack` | JSON `{through}` | both |
| `0x07` | `awareness` | JSON `{participant_id, name, colour_index, file?, anchor?, head?, preview_page?, following?, seq}` | both (hub fans out) |
| `0x08` | `blob_want` | JSON `{sha256, offset, length}` | both |
| `0x09` | `blob_chunk` | 32 bytes sha256, varint offset, bytes data | both |
| `0x0A` | `preview_subscribe` | JSON `{source?, have_fonts}` | iPad → Mac |
| `0x0B` | `preview_frame` | one display-list-v3 frame, verbatim | Mac → iPad |
| `0x0C` | `compile_report` | JSON `{state_vector_digest, read_set_digest, page_hashes}` | Mac → all |
| `0x0D` | `leave` | JSON `{reason?}` | both |
| `0x0E` | `error` | JSON `{code, message}` | both |

- `doc`: `u8 0` (the file map) or `u8 1` then a file-id (a text file).
- `sections`: varint n, n × {doc, varint m, m × op}, where op is a file op for the file map and a
  text op otherwise. A sender puts the file map first (it creates the text documents); a receiver
  must still accept any order.
- Participant ids in JSON are 16 lowercase hex digits (JSON numbers cannot carry 64 bits). A JSON
  position is `{replica?, counter?, assoc: "before"|"after"}`; no replica and counter means no anchor.
- JSON readers ignore keys they do not know. An unknown kind is answered with `error`
  (`code: "unknown_kind"`), not by closing the connection.
- `update.seq` numbers a sender's outbox; `ack.through` acknowledges every update up to it.
- Sync: `sync_request` carries the requester's state vector per document; the reply carries every
  unit not covered, in a causal order (the sender's application order, trimmed per §2.2).

## 5. Digests

FNV-1a 64 (offset `0xcbf29ce484222325`, prime `0x100000001b3`), integers little-endian.

- **Text**: for each stored scalar in document order (tombstones included): replica u64, counter
  u64, deleted u8, scalar u32.
- **File map**: for each entry in file-id byte order: file-id, kind u8, path stamp lamport u64 and
  replica u64, path length u64, path bytes, then `u8 0` or `u8 1` + blob stamp lamport u64 and
  replica u64 + sha256 + bytes u64 + media type length u64 + media type bytes, then deleted stamp
  lamport u64 and replica u64, deleted u8.
- **Project**: file-map digest u64, then for each text file in file-id order: file-id, text digest u64.

Equal digests mean equal structure (order and tombstones), not only equal text.

## 6. Fixtures

`crates/collaboration-core/tests/fixtures/collab-v1/*.json`, format `collab-v1-fixture/1`:
`{name, description, generator, frames: [hex of update frames], orders: [[frame index…]…],
expected: {project_digest, filemap_digest, files: [{file, path, kind, text?, digest?, sha256?}],
rejected: [{doc, replica, counter, kind}], pending}}`. Each order (forward, reversed, shuffled with
duplicates; forward only where the outcome depends on arrival, as for id conflicts) delivered to a
fresh replica must reach `expected`: the same state, the same distinct refusals (`doc` is `files` or
the file-id in hex; `kind` is `malformed`, `id_conflict`, `unknown_file`, `document_full` or
`pending_full`), and the same number of operations still waiting. Every frame must re-encode to the
same bytes. Besides the scenario fixtures there are `hostile-paths`, `rejected-ops` and the legacy
suites restated in collab-v1 (`legacy-*`: `adversarial.rs`, `interrupted_delivery.rs`,
`id_reuse_divergence.rs`).

`tests/fixtures/collab-v1-bulk/bulk-200.json` (format `collab-v1-bulk/1`, `{scenarios: [fixture…]}`)
is the bulk differential: 200 random simulation scenarios (two to five peers, edits, undo and redo,
file and blob operations, refused attacks, partitions) that the Swift suite replays in all three
orders. The oracle's test fails if the committed file differs from what it generates. A larger run on
demand: `FLASHTEX_COLLAB_BULK=<n> FLASHTEX_COLLAB_BULK_OUT=<file> cargo test --release -p
flashtex-collaboration-core --test v1_fixtures bulk`, then `FLASHTEX_COLLAB_BULK_FILE=<file> swift test
--filter FixtureTests/testBulkDifferential` in `apps/mac`. The oracle
writes the unprefixed fixtures (`FLASHTEX_COLLAB_RECORD=1 cargo test -p flashtex-collaboration-core
--test v1_fixtures`); the Swift core writes `swift-*.json`, which include undo and redo operations
(`FLASHTEX_COLLAB_RECORD=1 swift test --filter FixtureTests` in `apps/mac`). Both test suites replay
all of them.

## 7. Session layer (P1)

Status: **P1** (lane LIVE-SHARE-P1). Product: `apps/mac/Sources/FlashTeXCollabSession` (Swift, MIT;
Foundation, Network, Security and CryptoKit, no AppKit). The Rust oracle does not implement this
layer; the Swift tests (`FlashTeXCollabSessionTests`) run it over real loopback TLS.

### 7.1 Transport

- TCP to the hub, **TLS 1.3 only** (no resumption, no tickets). The hub presents a self-signed
  certificate over a P-256 key made in memory for the session (nothing goes to a keychain). The guest
  accepts exactly the certificate whose SubjectPublicKeyInfo SHA-256 equals the invite's `fp`; any
  other certificate fails the handshake, before an application byte. No client certificate: the guest
  proves the invite in `join`.
- Bonjour service `_flashtex-collab._tcp`, instance name = the session id, TXT `v=1` only (no project
  name: the network learns only that a session exists).
- Frames as §3, at most about 1 MiB of operations per `update` or `sync_reply` (a large sync is split
  into several frames; receivers integrate each as it comes).

### 7.2 Invite

`flashtex-collab://join?v=1&s=<session id>&k=<secret>&fp=<pin>&name=<project>&port=<port>&addr=<ip,…>&host=<name>`

`k` is 32 random bytes and `fp` 32 bytes, both base64url without padding. `addr` (comma-separated
addresses) and `host` (the hub machine's host name) are optional; a guest connects to `127.0.0.1` when
`host` names its own machine (two app instances on one Mac), then to each address, then to the Bonjour
service. An invite is **single use** and expires after **10 minutes**; the hub consumes it on the first
valid proof, whatever the hub user then decides.

### 7.3 Join

1. Guest → `join {invite_proof, nonce, guest_pubkey, display_name, device_kind}`. `nonce` is ≥ 16
   random bytes and `guest_pubkey` a Curve25519 signing public key, both base64url; `invite_proof` is
   base64url(HMAC-SHA256(k, nonce ‖ guest_pubkey)) over the decoded bytes.
2. The hub user approves (edit or view) or declines; the hub waits up to 120 s.
3. Hub → `join_ack {participant_id, role, token, colour_index, pins, environment_digest}`, then its own
   `sync_request` and `awareness`. The participant id is a random 64-bit value, unique in the session,
   and is the guest's replica id. The guest answers with `sync_request` and its outbox.
4. Reconnect: `join` with `token` (and an empty proof) rebinds the same participant and replica; a new
   connection for that token replaces an older one.

A `join` must arrive within 10 s of the TLS handshake, and until a connection has joined the hub
accepts frames of at most **4 KiB** (a larger one is answered with `bad_frame` and closed, never
buffered). The hub accepts at most 4 connections from one address at a time and at most 5
participants, itself and joiners still awaiting approval included. Colours are assigned in join order,
the hub having 0, modulo 8.

### 7.4 Operations

- **Replica binding** (§2.1): the hub drops every operation from a connection whose id's replica is not
  that participant's, and every operation from a viewer, before the CRDT; it relays what it kept to
  every other joined connection as an `update`. A guest has one connection, to the hub.
- **No file-map operations from guests (P1).** Only the hub creates, renames or deletes files; the hub
  drops every file-map operation a guest sends, whatever its ids, before the CRDT, and relays none.
  A host writes only the files it shared, at the paths it shared them under (never a path taken from
  the CRDT), and never a dotfile, a path through a symbolic link, or a path that collides with another
  case- or normalisation-insensitively.
- **Outbox**: a guest numbers its `update`s from 1 and keeps them until an `ack {through}` covers them;
  on reconnect it resends them after `sync_request`. Duplicates are harmless (§2.4).
- An integration that reports `pending_full` makes the receiver send `sync_request` to that peer.

### 7.5 Presence

`awareness` is sent on a change (at most every 50 ms) and as a heartbeat every 3 s; a peer's presence
is dropped 10 s after its last message. The hub passes at most 25 per second per participant (a token
bucket); the excess is dropped, not fanned out. `file` is the text file's id in hex (32 digits). The hub
overwrites `participant_id`, `name` and `colour_index` with the values it assigned before it fans an
`awareness` out, so a guest cannot impersonate another.

### 7.6 Errors

`error.code` values the session layer sends: `invite_invalid`, `join_denied`, `token_invalid`,
`session_full`, `join_timeout`, `removed` (each ends the guest's session for good), and `not_joined`,
`already_joined`, `unknown_kind`, `bad_frame`, `unexpected`. `leave` from the hub ends the session for
every guest.
