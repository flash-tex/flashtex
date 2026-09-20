# Capture bridge and reviewed insertion v1

Owner: Commander, FT-007. Updated September 19, 2026 (caret-mode destinations, approval-time wrap). Status: implemented local
Rust bridge interface, additive to [runtime-v1](runtime-v1.md); native integration
and encrypted nearby transport remain outstanding. Implementation and tests:
[`crates/bridge`](../../crates/bridge/README.md). This document governs these new
message types once merged; it does not change compilation messages.

The Mac launches a separate bridge process with a private application-data journal.
UTF-8 JSON Lines use the runtime envelope `{protocol_version:1,id,type,payload}`.
Each request ID is a nonempty string of at most 128 bytes; replies preserve it.
Errors carry `{code,message}`. Unparseable/unidentifiable requests use null ID.
Each line, including newline, is at most 12 MiB. Logs belong on stderr. Process
requests off the main UI thread; conversion can take up to 90 seconds and must not
block compiler/editor responsiveness. One process owns a journal at a time.

## Documents and destinations

All offsets are zero-based, end-exclusive UTF-8 byte boundaries. Document text is
at most 8 MiB. Project/capture/destination IDs contain 1–128 ASCII alphanumerics,
`-` or `_`. Paths are normalized relative paths; reject absolute paths, empty or
parent segments, backslashes, colons and NUL. Revisions are increasing integers.

| Request type | Payload | Reply type / payload |
|---|---|---|
| `document_open` | `project_id,path,revision,text` | `document_opened` / `{}` |
| `document_edit` | `project_id,path,base_revision,revision,start_byte,end_byte,replacement` | `document_updated` / `{revision}` |
| `destination_pin` | `destination_id,project_id,path,revision,start_byte,end_byte` | `destination_pinned` / anchor below |

An anchor contains `destination_id,project_id,path,pinned_revision,current_revision,
start_byte,end_byte,valid,binding`. The immutable `binding` contains original
`project_id,path,revision,start_byte,end_byte,source_sha256` and is durably bound
to each capture. Restoring a reused ID against another project, path, range or
source hash is rejected. Rehydrate the original snapshot/anchor, then replay known
edits to rebase; journal records without this binding require reselection. Capture `base_revision` means the original pinned
revision, not a guess from the phone. A document edit before the target rebases
its offsets. An intersecting edit, or insertion exactly at an empty target,
invalidates the target rather than guessing affinity. Unknown full-snapshot
changes invalidate affected anchors. A new selection receives a new destination
ID. The Mac supplies the current destination to the paired companion; users
should not have to type IDs or revision numbers.

## Durable capture and conversion

`capture_submit` retains the runtime-v1 payload. Accept only fully decodable PNG
or JPEG, at most 8 MiB encoded image bytes, dimensions at most 8192 × 8192 and a
64 MiB decode allocation limit. Instructions are at most 4096 UTF-8 bytes. Send
one image per capture. Invalid images receive an error, not an acknowledgement.

`capture_received` contains `capture_id,durable:true,has_proposal,applied`. The
journal has been atomically replaced and fsynced before acknowledgement. Identical
capture-ID retries return the existing record; a different payload using that ID
returns `capture_id_conflict`. Durable receipt does not mean conversion or insertion.

`capture_convert` payload is `{capture_id,supported_features:[]}`. The feature list
comes from actual compiler capabilities, at most 64 entries of 128 bytes each.
The bridge resolves the current anchor and assembles bounded context: selected
source, nearby source, package/macro definition line excerpts, project/path and
revision. Total source context is at most 16 KiB; selected text at most 8 KiB;
prefix/suffix at most 4096 bytes each. Excerpts are not complete macro analysis.

`capture_proposal` returns `capture_id,latex,ambiguities,required_dependencies,
context_revision`. LaTeX is 1–65536 UTF-8 bytes; each list has at most 32 strings,
each at most 2048 bytes. The UI displays uncertainties and dependencies explicitly.
No unsupported package is silently installed. Actual compiler validation of the
proposal in document context remains an integration gate, not an implemented
claim. Show the proposed edit and resulting diagnostics before approval.

Grok is opt-in at process launch (`--enable-grok`) and per conversion request.
The Mac credential adapter supplies its authorized key; never send keys from the
companion or store them in the capture journal. No automatic retry or provider
fallback occurs. Successfully journaled proposals are reused. A process crash
between a successful API response and journal persistence can require another
paid call; receipt deduplication does not promise exactly-once external billing.

## Review, insertion and crash reconciliation

| Request type | Payload | Reply |
|---|---|---|
| `capture_reject` | `{capture_id}` | `capture_rejected` / `{capture_id}` |
| `capture_prepare_insert` | `{capture_id,expected_revision,approved:true}` | `capture_edit` / prepared edit below |
| `capture_applied` | `{capture_id,edit_id,new_revision}` | `capture_application_received` / same fields |
| `capture_status` | `{capture_id}` | `capture_status` / `{capture_id,proposal,prepared,applied,rejected}` |

Rejecting a capture is durable and terminal. Retries are harmless; conversion and
preparation subsequently fail. Rejection after an edit has already been issued
is refused: first reconcile the Mac edit ledger, because an issued edit might
already have been applied. A new attempt uses a new capture ID.

A prepared edit contains `capture_id,edit_id,project_id,path,expected_revision,
start_byte,end_byte,removed_text,replacement,document_before_sha256`. It is
persisted before being returned. Preparation never edits source. Repeating the
same request returns the same edit only if the current source revision/hash still
match. Changing the source after preparation requires reconciliation, not blind
re-preparation or replay.

The Mac must:

1. Require explicit review approval for the currently displayed proposal and target.
2. Compare revision, SHA-256 of UTF-8 source, scalar boundaries and removed text.
3. Persist an edit-ID application ledger with its document transaction; apply one
   undoable source edit. An already applied ID must never produce another edit.
4. Send `capture_applied` after its source transaction is durable. Do not also send
   `document_edit` for that same edit: confirmation updates the bridge snapshot.
5. On disconnect/restart, consult `capture_status` and its own ledger before doing
   anything. Reopen the pre-edit snapshot before replaying a missing receipt, then
   synchronize the current source; never reapply to an already changed document.

The bridge accepts matching receipt retries and rejects conflicting IDs/revisions.
Its journal alone cannot make a separate native document store transactional.
Native ledger, undo integration and restart reconciliation require integration
verification. Anchors/documents are in memory and must be resynchronized after
restart. A stale/invalid destination must show a reselection action; the initial
bridge requires a new capture ID/destination for the renewed attempt.

## Nearby transport boundary

This executable implements local stdin/stdout, not a network listener. Planned
nearby transport must use explicit pairing plus authenticated encryption; Bonjour
is discovery only. Do not expose this plaintext process protocol to an anonymous
LAN port. The authenticated adapter forwards capture messages to the local bridge
and returns durable acknowledgements. Camera/Pencil device tests, disconnect
recovery, pairing, TLS identity storage and native Keychain access are outstanding.

Consumers: FT-003 Mac owns UI review, document transaction and bridge lifecycle;
FT-004 companion owns image production and authenticated delivery; FT-012 validates
published envelopes; FT-007 owns Rust journal, anchors and provider conversion.

## Dependency-bound proposal freshness (additive bridge update)

Conversion context now includes `dependencies`, a deterministic path-ordered list
of `{path,revision,source_sha256}` for the target and its literal include-connected
uploaded snapshots. Hashes cover each complete UTF-8 snapshot, including source
outside the bounded excerpts. All entries belong to `context.project_id`. The
journal persists these fingerprints together with the proposal. The context's
`definitions` contains complete supported lexical declarations with source path,
revision and line provenance, plus explicit lexical-analysis/omission notices.
It is not a declaration of evaluated TeX scope or full compiler support.

Preparation checks that the current dependency set, revisions and hashes still
match the saved conversion. A changed/missing dependency or legacy context without
fingerprints returns `proposal_context_stale`, even if the selected document itself
is unchanged. The UI must discard approval and explicitly offer conversion again,
then display the refreshed proposal for new review. `capture_convert` reuses a
proposal only when its dependency fingerprints and supplied supported-feature list
still match; an explicit conversion request refreshes a stale unprepared proposal.
There is no background provider call. A failed conversion keeps the old proposal
journaled but stale and unpreparable. Unrelated disconnected project files do not
invalidate a proposal.

An already prepared edit cannot be superseded by another conversion. When its
context becomes stale, preparation returns `proposal_context_stale` with receipt
reconciliation instructions. `capture_convert` returns that existing issued
proposal without calling the provider; `capture_status` exposes its prepared edit.
The Mac must reconcile its durable edit ledger before starting a new capture.
Receipt acknowledgement is still allowed: it reports an edit already applied,
not authorization to apply a new edit to changed source.

After restart, reopen the original target snapshot/anchor and all relevant current
project snapshots before review. Reusing a revision number with changed dependency
content is detected by the hash. Mac review UI integration and temporary-project
compiler validation remain separate acceptance gates; a fresh proposal is not proof
that the proposed TeX compiles successfully.

## Caret-mode destinations and approval-time wrap (additive bridge update)

Two optional fields, both ignored by older peers and absent from every existing
message when not used; `protocol_version` stays 1.

`destination_pin` accepts `mode`: `"fixed"` (the default; every rule above) or
`"caret"`. A caret anchor is the Mac's *automatic* destination — "wherever the
caret is when the capture is inserted" — pinned on the companion's behalf when
it asks `hello`/`destination_query`, never by the user. It differs from a fixed
anchor in exactly four ways: a `document_edit` never invalidates it (text
inserted at or before it shifts it, like a caret; an edit spanning it collapses
it to the end of the replacement); the same `destination_id` may be re-pinned
at any range and any revision (the id is the companion's handle, not a promise
about bytes — after a bridge restart the Mac re-pins it wherever the caret is);
a capture's `base_revision` and the binding journaled at receipt are
informational rather than checked; and `capture_prepare_insert` does not require
the conversion's dependency fingerprints to be fresh (the caret context is
re-derived at the anchor's current position when the edit is prepared, and the
whole replacement is gated against it). A fixed anchor may be replaced by a
caret anchor under the same id only when it is already invalid (the Mac's
"Insert at caret" recovery) or when the replacement is itself caret-mode; a
valid fixed anchor is still `destination_conflict` unless re-pinned identically.
The anchor reply carries `mode`. Unknown-snapshot `document_open` still
invalidates every anchor of that document, caret ones included.

`capture_prepare_insert` accepts `wrap: {prefix, suffix, kind?}` (each string at
most 1024 UTF-8 bytes, no NUL; `kind` is a free label such as `display_math`,
`inline_math`, `as_is`, at most 32 bytes, not interpreted). The Mac computes it
from the caret's context at approval time — the math delimiters and line breaks
that make the journaled proposal legal where it is landing — and the bridge
journals it with the prepared edit: `replacement = prefix + proposal + suffix`,
`capture_edit.wrap` and `capture_status.prepared.wrap` return it, and
`capture_application_received.wrap` echoes the wrap the applied edit was
prepared with. The whole replacement is gated by the same caret-context
predicate as the bare proposal (`unsupported_construct_requires_confirmation`
when it would nest or unbalance math); an oversized wrap is `invalid_wrap`. An
idempotent repeat of the request returns the journaled edit and wrap, whatever
wrap the repeat carried. The journal integrity check now requires
`replacement == wrap.prefix + proposal.latex + wrap.suffix`; the reviewed text
is still the journaled proposal and only the journaled proposal.

## Compiler validation before review approval

`capture_validate` payload is `{capture_id,expected_revision}`. Configure the bridge
with `--compiler ORIGINAL_FLASHTEX_BINARY --compiler-entry main.tex`; both flags
are required together. Without them, this request returns `compiler_not_configured`.
The selected binary is the original Rust compiler, never a reference LaTeX engine.

Reply type `capture_validation` contains `capture_id,context_revision,validation`.
The validation object contains request correlation, project/revision, original
snapshot path/revision/hash identities, hypothetical snapshot identities, compiler
`status,diagnostics,pages`, `source_is_hypothetical:true`, and an explicit compiler
compatibility limitation. Source offsets refer to the hypothetical snapshots;
they must not navigate unchanged editor text without translating the proposed edit.

Validation requires a current, non-rejected, unapplied proposal with matching
context dependency fingerprints and current target revision. It constructs copies
with the proposed replacement, compiles those copies, and checks reply ID, project,
revision, UTF-8 spans and display geometry. It never prepares an edit, changes
source, saves approval or calls Grok. Review approval and subsequent insertion remain
separate. Failed/recovered compiler output is evidence to display, not successful
full-compatibility certification. The current compiler's multi-file limitations
remain visible in diagnostics.

Compiler execution has a five-second deadline, an 8 MiB request/response bound and
a 64 KiB stderr bound. File-backed transport avoids pipe backpressure deadlocks;
file size is polled, so a misbehaving configured binary may transiently write more
than the cap before termination. Output is never loaded past the bounded read.
Run bridge requests away from the Mac UI thread. Compilation for live editor
preview remains a separate process; proposal validation must not block typing.
