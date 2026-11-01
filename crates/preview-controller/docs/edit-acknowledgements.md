# Optional metadata-only ordinary edit acknowledgement

An ordinary `edit` request may include `response_mode:"metadata"`. Omitting it or
using `"full"` retains the existing full Document response. Unknown/non-string
values reject before durable source mutation. The grouped-edit extension is described below. This option does not apply to
capture approval replies. Permanent command semantics are unchanged.

A metadata result echoes `response_mode:"metadata"` and contains `document` with
`project_id`, `path`, durable `revision`, `source_sha256` and UTF-8 `byte_length`.
It omits `text`, and retains `preview_error` and `save_and_submit_ms`. A preview
failure still permits a successful durable edit acknowledgement. The response is
an acknowledgement of saved source, not evidence of current preview paint.

Native callers must retain the exact submitted source until reconciling its
acknowledgement. Verify session/request/project/path/revision/hash/byte count;
do not overwrite newer local typing when an older acknowledgement arrives. On a
lost reply, reopen/query the authoritative document and compare its identity before
retrying. Ordinary edits remain revision/hash guarded, not permanent-ID operations;
replaying the old expected revision rejects after a successful save. The existing
permanent-ID semantics of reviewed/grouped operations are unchanged.

Older helpers may ignore the optional request field and return the full Document.
Accept that existing response when valid; never resend a successful edit merely
because the metadata echo is missing. Adoption in the native app is still pending.

The ledger now exposes `replace_document_in_place` for this path, reusing the same
history recording and durable commit without cloning a response Document. The
existing `replace_document` API calls that path and clones the saved result for
legacy callers. The controller shares indexing/compile completion logic between
both modes. Source compilation still needs its normal input snapshot; this change
does not claim all source copying is eliminated.

Actual helper checks cover a500KB Unicode edit, metadata frame below1KB, exact
reopen/hash, premutation invalid-policy rejection, stale retry, default full reply,
and an unconsumed application ACK followed by kill/reopen/reconciliation. Existing
ledger history, receipt, checkpoint, crash and helper/compiler suites pass.

The isolated paired wire probe in `../benchmarks/edit-ack-metadata` encodes the same
500KB Unicode source: full reply500,288bytes versus metadata326bytes. Full encoding
0.250–0.574ms versus metadata0.00045–0.00190ms in ten alternating pairs. This is
synthetic response encoding only, not ledger fsync, indexing, native IO or paint.


## Grouped-edit metadata acknowledgement

`apply_group` also accepts the same top-level `response_mode:"metadata"`; the
existing `command` payload and its fingerprint do not change. The result echoes
that mode and returns `history` containing a metadata-only `document`, original
`command_revision`, `replayed_command`, `can_undo` and `can_redo`, plus the usual
preview error and save timing. Omitted/`full` mode retains the full history result.
Invalid modes reject before mutation or command-ID processing.

Exact retries use the same permanent command ID and identical command payload,
even after a lost reply/restart. A changed command under that ID rejects. Do not
mint a new ID merely because a reply was lost. The document in a replayed result
is the current durable document: after undo it can have revision3 while the old
command_revision remains2. That response does not mean the old edit was reapplied.
If current source has advanced, reconcile through the existing document/snapshot
query before applying any UI replacement or undo action.

Metadata projection now uses the ledger's source-free HistoryCommandStatus and
borrows the authoritative current document for indexing. Existing full-result APIs
wrap the same mutation path and clone the document only for callers requesting it.
History snapshots and permanent-ID records remain unchanged; this does not claim
zero-copy mutation. No new patch format, cap or source-approval shortcut is
introduced. Native adoption remains opt-in and has not been measured here.

An actual helper test starts with500KB of Unicode source, applies two byte-aligned
edits as one group, loses the application ACK, reopens and retries exactly. It
compares full and compact history flags/hash, rejects an ID conflict, undoes the
whole group and retries the old command again: command revision2/current revision3
and redo availability remain correct. The metadata response remains below1KB.

The grouped500KB synthetic wire probe is recorded separately in
`../benchmarks/group-ack-metadata/provenance.json`; it retains all history flags
and command revision. This measures reply serialization only, not native or
ledger performance.

## Undo and redo metadata acknowledgement

`undo` and `redo` accept the same response policy and compact `history` shape as
`apply_group`. Validation occurs before mutation. Their existing command payloads,
fingerprints, permanent IDs and full-response defaults are unchanged. The shared
controller adapter and source-free ledger status avoid response source clones
while preserving legacy full results and durable snapshots.

A retry of an old undo or redo returns its original command revision alongside the
current document identity and current undo/redo availability. It does not replay
the source change. Clients must reconcile a newer document rather than reconstruct
it from the old operation alone. Changing response mode does not change command
identity; changing command fields under an existing ID still rejects.

The real helper test exercises undo, redo with an unread application ACK, restart,
exact redo retry, a conflicting ID, a stale new undo, and both old commands after a
later ordinary edit. Full and compact replies agree on current hashes/history
flags; the 500KB redo response is below1KB and reopening preserves exact text.
This is protocol/recovery evidence, not native undo integration or paint timing.

## Source-free ledger history status

`apply_group_status`, `undo_status` and `redo_status` reuse the existing command
validation, mutation, commit and permanent receipt paths. They return command
revision/replay and current undo/redo flags without a Document. The helper then
borrows `Store::document`; there is no intervening mutation. Legacy APIs return
unchanged full HistoryResult values. Disk format, snapshots and retention do not
change.

A paired test starts from an identical durable store, compares full/status group,
undo and redo, reopens after another edit, retries all three commands and checks
conflict/stale rejection plus exact final persisted bytes. The initial comparison
fixture used independently generated store IDs; it was corrected to seed both
stores from the same durable state before comparing bytes.

The single paired500KB durable undo allocation observation in
`../../edit-ledger/benchmarks/history-command-status/provenance.json` counts31
allocation/reallocation requests totaling5,002,401bytes for the full API versus27
requests totaling4,502,328bytes for status. The500,073byte difference is the removed
response Document clone. These are cumulative allocation requests, not peak memory,
wall-clock latency or native paint. History/commit allocations remain.

## Parsed edit source ownership

The helper moves the already parsed JSON `text` String into the existing durable
edit API for both full and metadata replies. It parses response policy, path,
revision, hash and text type before taking that String. Invalid input remains
untouched; source revision/hash and project membership are still checked by the
same controller/ledger paths. Envelope identity is checked before dispatch. The
small optional source-binding token remains owned across the mutable dispatch so
historical bindings retain their existing behavior.

The500KB parsed Unicode request test keeps the prior `.to_owned()` control alive:
that control has a distinct500,000byte buffer, while the new input retains the
exact original parsed pointer and500,000byte capacity. This proves elimination of
that one source clone. It is not a measurement of aggregate allocations or latency;
small path/hash/token copies and durable/index/compiler allocations remain. The
test also checks invalid fields leave the request intact and omitted/full response
policies preserve legacy selection. Full stdio30/30 and strict lint pass.

## Parsed history command ownership

The helper now transfers the owned JSON `command` value into the existing typed
GroupedEdit/HistoryMove deserializer for group, undo and redo. It checks operation,
response policy and path before taking the command; typed deserialization completes
before calling the unchanged controller/ledger mutation. A malformed typed command
may be consumed from the ephemeral request on rejection, but never reaches durable
mutation. Session/request identity and source-binding token remain intact.

The64KB Unicode replacement test compares the old Value-clone control with the
moved command: the typed replacement retains the exact parsed pointer, the control
has a distinct buffer, and reserialized typed command values match. Undo/redo
selection and default full mode are unchanged. This proves removal of that copied
replacement buffer, not aggregate allocation or native response time. Existing
permanent-ID conflict/retry/recovery and stale source guards still apply.
