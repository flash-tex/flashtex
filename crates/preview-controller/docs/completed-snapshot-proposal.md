# Completed snapshots during continuous typing — integration proposal

Status: internal runtime/controller opt-in prototype implemented; native negotiation and activation remain pending. Existing strict current-preview behavior remains the default. Evidence: `benchmarks/helper-burst/50k.json` has20 durable edits at30ms intervals but only1 preview,11 stale completions and8 superseded requests. The final exact preview arrived91ms after the final send. Publishing already-completed older snapshots could show progress during this burst; it cannot establish instantaneous current-document output or solve compiler throughput.

## Why this needs a runtime change

`document-runtime::Session::poll` validates every complete response against its original request, then drops the result when a newer revision is pending, emitting only `Event::Stale { id, revision }`. The controller cannot recover those bytes from that event. `Controller::submitted` also retains only the latest request binding. A controller-only switch that relabels an old result as current would be incorrect.

Keep `Event::Preview`, `Controller::is_current_preview`, and default helper `kind:preview` semantics unchanged. Add a separate runtime opt-in side channel for fully validated, uncancelled stale completions. A proposed `take_completed_snapshot()` moves out at most one retained result; it never reparses, clones or recompiles it. Disabling the feature, closing the project, cancelling its request, restarting the compiler or changing session identity clears retained snapshots. A higher compile revision replaces a lower retained snapshot. No partially framed or validation-failed response can enter this slot.

The result needs immutable origin metadata: project ID, session/incarnation, request ID, compile generation, and an opaque bounded source-binding token captured on submission. The controller must pass that token with the exact original `VersionSnapshot` rather than derive it from whichever document is current at completion time. Runtime should return the token unchanged; it must not depend on editor-specific revision types. Existing callers omit the optional token. Use an owned byte/string token with an explicit small limit, not arbitrary JSON or captured document text.

For a controller integration, retain only bounded metadata for potentially deliverable submissions and remove it on supersession/cancellation/completion. Do not introduce an unbounded request-ID map. Size this against actual runtime admission/event bounds; reject admission before a durable mutation if additional metadata capacity is required, or discard historical display eligibility without compromising the successful source save. Never reject a source save merely because an optional historical preview cannot be retained.

## Proposed helper negotiation and event

Native explicitly requests `completed-snapshots-v1`; absence preserves current wire behavior. The helper must report negotiated support, and must not emit the new event before acknowledgement. This is separate from layout capabilities (`rules-v1`, `font-hints-v1`). Old native clients receive no new message kinds.

A historical update would have this shape:

```json
{
  "protocol_version": 1,
  "session_id": "current-helper-incarnation",
  "id": null,
  "type": "update",
  "payload": {
    "kind": "completed_snapshot",
    "request_id": "preview-10",
    "compile_revision": 10,
    "source_versions": {"main.tex": 7},
    "current_compile_revision": 12,
    "is_current": false,
    "source_actions_enabled": false,
    "result": {"...": "the unchanged, fully validated original compile result"}
  }
}
```

The illustrative result placeholder above is not a valid compiler response. The implementation must forward the full original result through the owned-value wrapper path. Source versions must be the originating versions, not the latest ones. Session identity and project identity must be checked independently of numeric generation, which may restart in a new helper.

## Native behavior and authorization boundaries

The app may paint a monotonic sequence of completed snapshots while it visibly indicates that a newer edit is compiling. Recheck current native session/project and latest displayed generation immediately before painting, including after queued UI work. An older completed generation must never overwrite a newer displayed generation. Closing/reopening or switching documents invalidates queued snapshot deliveries.

A historical snapshot grants no source navigation, diagnostic jump, AI edit destination, source-map application or current-document export authority. Disable these actions for historical display; restore them only after the existing exact current-source checks succeed. Do not use an old source span with the current text merely because its byte offset remains in bounds. Export can either wait for the current build or offer an explicitly versioned historical artifact through a separately reviewed flow; no silent substitution.

Keep at most one historical pending frame at each integration boundary. Bounded memory includes runtime retained Value, helper serialization buffer, output queue, and native retained PDF/display-list caches. Existing output limits and stalled-reader termination remain in force. Do not allow a stream of optional historical updates to crowd out durable acknowledgements or final current previews; prioritize those and drop optional snapshots before queue admission when required. This likely requires explicit scheduling support beyond merely appending another FIFO event.

## Gates before enabling the feature

| Gate | Required evidence |
| --- | --- |
|Default compatibility| Existing callers receive identical event kinds, exact compiler bytes and current-source rejection behavior with feature off. |
|Validated origin| Slow A followed by B can deliver A only as historical, bound to A's immutable source metadata; malformed A produces no snapshot. |
|Cancellation/restart| Explicit cancellation, project close, capability reset and session restart prevent late delivery; same numeric generation in a new session cannot authorize an old frame. |
|Monotonic UI| Delayed historical UI callbacks cannot replace a newer displayed/current frame. |
|Source actions| Historical output cannot initiate navigation or an edit; old diagnostic spans remain inert even if valid offsets in current text. |
|Bounded delivery| Flood edits and stall native reads; acknowledgements retain their durable semantics, final current preview is eligible, optional snapshots cannot grow queues without bound. |
|Typing fidelity| Repeat5/50/500KB burst driver with exact final clean equality, durable reopen, actual frame timestamps and number of updates during typing. Report historical lag separately from current-preview latency. |
|Native acceptance| Mac owner runs real rendering/interaction tests and marks older snapshots visibly. Linux helper events alone cannot prove native safety or perceived responsiveness. |

Runtime owner must approve and implement the additive origin-token/retained-snapshot API; controller owner implements bounded metadata and helper negotiation; Mac parent owns visible labeling, UI ordering and source-action gating. Publish interface agreement before parallel edits. Do not enable production behavior until all three owners complete the gates. In parallel, continue compiler dependency/incremental/layout work: this feature complements exact low-latency compilation and does not replace it.


## Runtime prototype checkpoint

`Session::set_completed_snapshots_enabled(bool)` toggles a separate one-result historical slot and invalidates in-flight origin tokens across policy toggles. `submit_with_snapshot_origin(request, capabilities, token)` accepts an explicit nonempty/control-free token up to1024bytes only when enabled. The existing submit APIs never enroll a request, even if retention is enabled. Token contents remain caller metadata: runtime captures and returns them with the exact original request/project/revision; the controller must bind them to immutable source versions and a fresh session incarnation before use. They are not authentication credentials.

`take_completed_snapshot()` moves out the single retained fully validated stale result. Regular `Event::Stale` still occurs; `Event::Preview` retains its strict current semantics. No result copy, new compiler protocol field, native helper message or new thread was introduced. Fresh current output for that project, project close, runtime failure and disabling clear the retained result. At most one result is retained across all projects; source text is not duplicated into this slot. Existing configured frame bounds still apply to its input; parsed Value allocation is not represented as a tight raw-byte memory bound.

Five new subprocess tests cover opt-in origin binding/take-once, default and ordinary-submit behavior, policy epoch invalidation, fresh/close clearing and malformed response rejection. The fake compiler gates later responses on an explicit release file so correctness does not depend on winning a timing race. Controller token/version bookkeeping, native negotiation/painting/source actions, queue priority, cross-session UI rejection and extended burst measurements remain required before activation.


## Controller prototype checkpoint

`configure_completed_snapshots(bool)` enables only the internal controller path. No stdio command or new wire event exists yet. Each successful compilation records the exact original `VersionSnapshot` against its fresh random controller incarnation, policy epoch and compile generation. Records are capped at64; evicting old optional eligibility never rejects or modifies a durable edit. Runtime-origin matching requires the original request ID, project and generation. Consumed or retired records are removed; raw source text is not duplicated into this metadata.

`take_completed_snapshot()` yields a separate `HistoricalPreview` with private provenance and read-only result/version accessors. `claim_historical_display(&snapshot)` rechecks incarnation, epoch, project and monotonic display generation and marks a successful claim once. Existing current Preview delivery advances the display floor so an older queued callback cannot repaint over it. Membership/layout/policy changes, restart, close and runtime failure invalidate historical eligibility. This claim is strictly for historical display and does not grant navigation, edit or export authority.

41 controller tests pass with strict all-target Clippy, including original compiler/stdout/backpressure recovery cases and three new lifecycle tests: original-version binding/current-output ordering; policy/layout/restart/close/other-controller invalidation;80 durable edits through the64-record metadata cap and exact reopen. Runtime's26-test checkpoint remains separately verified. Native consumer agreement, historical message scheduling/priority and real burst display measurements remain pending. Neither Rust API is enabled automatically for existing callers.

## Internal delivery scheduler checkpoint

`experimental_delivery::DeliveryQueue` is an isolated prototype, not connected to
stdio. It reserves bounded required-frame slots and payload allocation capacity,
keeps durable/current frames in FIFO order, and retains at most one replace-latest
historical frame outside that budget. Rejected required admissions return the
original frame to the caller. Successful current admission evicts older history;
cancellation invalidates queued display work and late old-epoch arrivals while
preserving durable acknowledgements for draining. Close prevents new admission.
Frame payload and identity/origin allocation capacities are checked before retention.
Allocator bookkeeping and other process allocations are not included in this bound.

Five deterministic tests cover optional flooding, required slot/byte backpressure,
exact returned ownership, generation supersession, lifecycle invalidation, wrong
incarnation and oversized allocation rejection. Existing 41 controller tests also
passed with the prototype present, including real compiler and stalled-reader gates;
all-target strict Clippy passes after the allocation checks.

Priority applies only before dequeue: an already-started JSONL frame cannot be
preempted safely. The caller must validate serialized frames, preserve immutable
submission provenance, and recheck display identity/floor at consumption. This queue
alone does not establish end-to-end latency or paint safety. The negotiated helper
adapter must additionally drop history before admission when required output is
pending, reset negotiation on restart, and preserve the existing stopped-reader
failure bound. Native agreement is recorded in issue 2 comment 5644981151; activation
and actual Mac paint/source-action acceptance remain separate gates.

## Negotiated stdio adapter

The helper now supports a separate request, off by default:

```json
{"protocol_version":1,"session_id":"session1","id":"history-config","type":"configure_completed_snapshots","payload":{"capability":"completed-snapshots-v1","enabled":true}}
```

Wait for the matching `result` response carrying `capability` and `enabled` before
submitting work. Put an opaque `source_binding_token` of 1..128 UTF-8 bytes in the
payload of each `compile`, `edit`, or other request that submits compilation. The
helper captures that token against the exact generation admitted while handling
that request. It never supplies a token for earlier work or substitutes a later
request's token. Tokens are opaque (JSON-escaped controls are preserved), not
credentials. Invalid supplied tokens fail before request mutation. Missing tokens
leave that compilation ineligible for historical output without rejecting edits.

A `type:"update"`, `payload.kind:"completed_snapshot"` includes explicit session
and project IDs, original source versions and token, compile/current-compile
revisions, `is_current:false`, `source_actions_enabled:false`, request ID and the
full original validated result. In negotiated mode this optional frame replaces
its matching legacy stale notification; either can be omitted when optional
admission is unavailable. Other existing default-mode event semantics remain.

The adapter keeps the existing eight-slot required FIFO and two-second stalled
write handling. A separately bounded single optional frame is rejected while any
required response is queued or being written. New required output discards queued
history. The writer checks the required channel before claiming optional work;
serialization is skipped when required output is already pending, and admission
rechecks under the shared lock. This gives scheduling priority, not preemption of
an already-started frame or a hard end-to-end latency guarantee. Output remains
bounded by the existing 16 MiB JSONL serialization limit; optional overflow drops
without replacing a durable reply with an error. Source saving remains independent
of compilation completion.

`restart` and `close` disable negotiation and invalidate optional queued output;
new helper sessions also start disabled. Reconfiguration clears prior bindings.
The native consumer must still check session/project/token and monotonic generation
immediately at paint, visibly label historical output and disable all source actions
and export. No app default is activated by this helper change.

Validation: 52 tests including explicit original-compiler gates, full-size output,
EOF recovery and stalled readers pass, as does strict all-target Clippy. A controlled
Python compiler fixture gates individual generations and proves original opaque
UTF-8 token echo/source binding, newer current output, restart requiring negotiation,
and invalid token rejection before a durable edit. It is a transport/lifecycle test,
not compiler parity or a native paint benchmark. Two queue tests additionally cover
required pending/in-flight rejection, FIFO/backpressure and optional replacement/reset.
