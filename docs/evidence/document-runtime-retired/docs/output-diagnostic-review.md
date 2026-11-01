# Output diagnostics: independent review checkpoint

FT049r31. Read-only review of root proposal69bffa06 and in-progress helper files
`output_delivery.rs`, `main.rs`, `helper_replay.py`, `typing_burst.py`. No peer edits,
heavy tests or measured workloads. Final publication review remains pending.

Required diagnostic contract: numeric request/output sequences and source-free
scalar fields only; no external IDs, paths, source text or binding tokens. The
proposal's earlier request-ID logging language must be narrowed accordingly.

Findings communicated to helper owner:

- Optional eviction/replacement must name the original retained frame sequence.
  Required admission attempts create sequence gaps when refused; retrying returned
  bytes is a new attempt, not the same semantic identity.
- Required FIFO priority and required-inflight exclusion remain unchanged. An
  optional write already started is nonpreemptible. Watchdog timeout must name the
  active sequence and remain distinct from write_failed or write_finished.
- eprintln while holding queue state can stall admission/dequeue if stderr blocks.
  Move logging outside the critical section where feasible and document sink
  assumptions; diagnostic overhead is not zero.
- Required enqueue can wake receiver before sender emits admitted. Cross-thread
  stderr line order is not a strict lifecycle ordering proof; use numeric sequence
  and scoped timestamps, not adjacency or contiguous sequence assumptions.
- Receiver candidate decode timing initially included copying the captured raw
  wire before json.loads. An explicit decode_started timestamp is needed to label
  JSON-only duration; frame receipt includes existing buffer partition/copy work.
- Wrapper-local receiver sequence skipped startup reads consumed directly by
  snapshot_after_initial_preview. Use a Client-wide ordinal or preserve exact
  skipped prefix before pairing with full-session successful writes.

Receiver correlation must preserve actual byte/frame order while accounting for
replaced, evicted and refused offers. Helper and Python clock origins differ;
within-process durations cannot establish cross-process transport time without
explicit calibration. Successful write does not prove receiver decoding or native
paint. Completed15 from earlier capture remains an observed undelivered optional
result; new instrumentation cannot retroactively assign its cause.

Updated in-progress candidate inspection confirms queue diagnostic emission moved
outside state mutexes, watchdog carries active sequence with a distinct timeout,
and Client-wide receive ordinal plus explicit decode_started resolve the receiver
findings. Owner reports tests/lint; no independent execution was performed.

A remaining attribution gap was communicated: an evicted optional frame does not
reach the receiver, so class/byte count/output sequence alone cannot bind it to its
original compile generation. Optional outcome currently lacks that numeric binding.
Carry a scalar generation-to-sequence association or explicitly leave that stage
uncorrelated. Do not infer it from adjacent stderr records or equal frame lengths.
Request sequence saturation also repeats u64MAX, unlike output sequence's checked
exhaustion; use consistent exhaustion handling before claiming unique identities.

## Final published implementation

Read-only final review pinned to `e8b5a6fa`, after owner reported88555 terminal:
16 unit and30 stdio tests passed (including enabled actual-producer gates), strict
lint passed. These are owner execution results, not independently rerun tests.
No measured workload was executed for this review.

The final implementation closes the recorded findings. Historical generation is
captured before snapshot consumption, passed through `offer_with_generation`, and
retained in Frame with its numeric sequence. Admission, refusal, replacement,
eviction, dequeue and write traces therefore retain a generation even when the
frame never reaches the receiver. Optional serialization outcomes also name the
generation. Request sequence exhaustion now yields None instead of repeatingMAX.
Output sequence never wraps. Queue diagnostic emission occurs outside state locks.

Client-wide receive ordinals include startup frames; explicit decode_started
separates raw-wire capture from JSON decoding. Retained receiver records are capped
at4096 with an explicit dropped-record count. Active writer sequence is included in
watchdog timeout, distinct from successful write or write failure. Required FIFO,
required-inflight exclusion, nonpreemptible started optional write, frame/queue
bounds and diagnostic-off defaults remain unchanged by the inspected diff.

No new source, token, path or external-ID fields appear in the added scalar logs.
No remaining concrete review blocker was found. The documented clock-origin,
diagnostic-overhead, cross-thread log-order and receiver/native-delivery caveats
still apply; this is not a performance or native activation endorsement.
