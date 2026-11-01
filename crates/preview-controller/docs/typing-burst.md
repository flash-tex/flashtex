# Sustained 50KB grouped typing acceptance

`examples/typing_burst.py` schedules20 visible grouped edits30ms apart on the same
50KB source/release helper/release producer used in the single-edit experiment.
It records actual send and flush timestamps independently from the receiver;
predicted revision/hash guards are never silently repaired or retried on refusal.
All edits are scalar-aligned byte ranges after a UTF8 prefix, with permanent IDs.

The initial run acknowledged all20 edits but emitted only final current revision21,
130.94ms after the last send. Independent review requested explicit monotonic/current
preview checks and an overall deadline, plus binding the final metric to revision21
rather than whichever frame was last. The strengthened rerun also acknowledged20,
with only final revision21 appearing98.03ms after the last send. The two original
captures and exact captured harness versions are preserved separately in
`benchmarks/typing-burst-50kb`; they are not a calibrated statistical comparison.

In the checked run, sends stayed within0.23ms of the intended schedule and ACK
arrival latency was at most3.10ms. No current preview arrived during the roughly
570ms send window. Fast durable ACKs and a final preview below200ms therefore do
not establish continuously updating current previews while typing. This gap stays
in GH21; no native paint, arbitrary-document or worst-case guarantee follows.

Both runs verify final exact source/revision21 after killing and reopening the
helper. A clean invocation uses the original captured final compiler request and
matches the actual current preview result. After measured typing, history contains
20 permanent IDs; retries of the first and last commands report command revisions2
and21 against current document21 without changing source. Changed fingerprint under
the first ID rejects. These post-measurement actions can request extra compilation;
they do not enter burst latency metrics. All recorded producer PIDs terminated.

The checked receiver refuses previews older than the latest delivered ACK and
nonincreasing preview revisions. An overall experiment deadline is checked around
each bounded reader call. Failure remains a failure rather than an extended timing
budget or permission wait. Original request/event/producer/clean-output/source and
phase artifacts are deterministically compressed with original and compressed hashes.

## Completed intermediate output and current-only delivery

`completion-disposition.json` audits the existing checked capture without running
another workload. It reconstructs every source revision from the original20 guarded
commands, verifies removed bytes and prior hashes, and binds each producer input and
emitted result to the helper event with the same request ID/generation.

Actual generations2,3,5,7,9,11,13,14,16,18 and20 completed successfully and became11
helper stale notifications. Eight pending generations were superseded. Only21 was
forwarded as a current preview. The lack of intermediate current previews is thus
not an absence of completed compiler results: completed results existed but no
longer matched the latest submitted source. This behavior preserves current-source
identity; the evidence does not justify calling those older results current.

An optional historical-display experiment can evaluate visible progress separately
with explicit stale labels, original source bindings and source actions disabled.
It must not change the default or claim current/native/every-keystroke acceptance.

## Explicit historical preview observation

`benchmarks/typing-burst-50kb-historical` uses the existing explicit
completed-snapshots-v1 option with one source-binding token for each burst edit.
It preserves the same source,20 command payloads (apart from those tokens), target
30ms schedule and release binaries. The default remains off and no native feature
is activated. Historical display is disabled again before old command retry tests.

All20 ACKs succeeded. Seven historical snapshots arrived at approximately86,172,
254,338,423,509 and595ms, binding editor revisions2,4,7,10,12,15 and18 respectively.
Six arrived before the last send at570ms. Each was explicitly is_current=false and
source_actions_enabled=false, with its exact original binding token and original
producer result/request source. Only revision21 was current, about98.92ms after
the last send. Clean final compilation, exact reopen and receipt/conflict gates pass.

The Python sender thread lagged its intended schedule by up to23.91ms, compared
with0.23ms in the earlier current-only capture. Maximum observed ACK arrival latency
was36.56ms. Receiving/decoding large historical frames can contend with the sender
in the same Python process; these measurements do not isolate that effect or
establish a matched-cadence latency tradeoff. They demonstrate source-bound older
visual progress, not current output for each keystroke or native paint performance.
A separately scheduled sender with deterministic lifecycle tests is needed before
any future controlled cadence comparison. No additional comparison run was made.

Post-capture review strengthened the harness to require the historical result ID
and compile revision to match its envelope and to locate a unique request/output
pair within the same captured producer stream. All seven original historical
results pass this audit; altered result IDs and compile revisions are rejected.
`correlation-review.json` records this later verification separately; original
measurement provenance and captured bytes remain unchanged.

## Separate-process sender (next experiment harness)

The harness now uses `scheduled_sender.py` rather than a receiver-process Python
thread. It inherits only the helper input descriptor, reads pre-encoded commands,
and records actual send/write-completion times on the shared monotonic clock.
The parent flushes before launch and sends no commands until this process exits.
A 200ms startup lead is not a scheduling guarantee; lateness must still be reported.

An absolute 30-second child deadline bounds waits and pipe backpressure. Writes
use at most PIPE_BUF bytes after writability, with one writer for the burst.
Parent cancellation and finish-timeout kill and reap the child, and normal cleanup
stops it before closing the helper. The parent's input descriptor remains usable
for subsequent receipt checks. No descriptor blocking flags are changed.

Five process lifecycle tests cover ordered original bytes, reuse of the parent's
writer, a closed reader, undrained-pipe deadline, cancellation and completion-timeout
reaping. These are harness checks, not a new compiler/native timing measurement.
The original archived threaded captures and their provenance remain unchanged.

Independent review added sender-failure polling before/after reads and on read
failure, so a recorded child error replaces a generic receiver timeout. Config and
result files are retained in the capture's sender directory, including failures.
Nested cleanup attempts helper shutdown and guarantees event-file closure even if
sender cleanup raises. If the helper kill/wait itself fails, Client.stop can leave
its descriptors unclosed; this is not an all-path helper cleanup guarantee. A forcibly killed parent is not an immediate child-death guarantee:
the child still has its absolute deadline; that case is not claimed by these tests.

A sixth lifecycle test transfers two large UTF-8/ASCII frames through many pipe
writes and fragmented reads, checking every byte and frame order. Child failure
polling does not interrupt an already-blocked helper read: its existing 15-second
read deadline can elapse before the recorded sender error is surfaced. This is
bounded eventual reporting, not an immediate child-exit wakeup.

## Controlled separate-process pair

`benchmarks/typing-burst-process-pair` preserves one current-only run followed by
one explicitly historical run of the same 20 edits/50KB source, release helper and
producer, and sender harness f240daec. Commands match apart from historical binding
tokens, and initial/final sources and final compiler result match exactly. Each
mode preserves original provenance plus compressed captures with original and
compressed hashes. Local reviewers confirmed no concurrent heavy jobs; this is
not an isolated-host or repeated statistical benchmark.

| Observed quantity | Current only | Historical opt-in |
| --- | ---: | ---: |
| Maximum send lateness | 0.105ms | 0.092ms |
| Maximum ACK arrival latency | 2.940ms | 39.268ms |
| Historical frames during sending | 0 | 5 |
| Total historical frames | 0 | 6 |
| Final current preview after last send | 80.341ms | 141.404ms |

Both runs acknowledged all20 edits, emitted only revision21 as current, and passed
clean final compilation, exact durable reopen, old permanent-ID retries and
conflicting-fingerprint refusal. Historical revisions2,5,9,12,18,20 retain original
source/result identity and disabled source actions. Original producer processes
were absent after cleanup. No further run was made.

The low sender lateness removes the earlier threaded sender's large scheduling
confound in this pair. The receiver still decodes frames in one process, so ACK
arrival measurements include its handling of earlier frames. Neither the observed
61ms final difference nor the ACK difference isolates a causal implementation cost.
Native paint and current output during every keystroke remain unproven, and the
historical feature remains off by default.


## Existing trace attribution and next instrumentation

`diagnostic-attribution.json`, reproducible with
`examples/burst_diagnostic_attribution.py`, verifies archived hashes before deriving
phase totals. The six delivered historical frames total9,892,397 bytes (~1.65MB
per frame). Six admitted optional serialization/offer calls take3.51–6.89ms each,
29.19ms total. That timing includes admission checks and queue offer; it does not
measure writer completion or receiver decoding. Request handling peaks around
2.6ms in either whole-session trace, which also includes setup and receipt checks.
Those samples have no request IDs or absolute timestamps, so they cannot be
correlated reliably with the maximum39.27ms ACK receipt observation.

The final runtime/controller totals are72.81/74.51ms in current-only mode and
135.40/137.04ms with historical output. These scopes overlap and include waiting;
they are not additive CPU costs or proof that historical serialization caused the
difference. The current writer preserves required FIFO priority but cannot preempt
an optional frame already being written. Existing telemetry records neither actual
write duration nor receiver decode duration, so serialization, pipe backpressure
and receiver parsing cannot be separated from these captures.

Independent audit also found producer revision15 completed in the historical run
without a delivered historical or stale notification. Optional output is lossy.
Six admitted offers do not prove why that other result was not delivered; the
outer eligibility/claim branches have no diagnostic outcome. No loss cause is
inferred from the missing frame.

Before choosing an optimization, the minimal diagnostic extension should:

- Attach a numeric diagnostic request sequence and a helper-relative monotonic
  timestamp to request timing; do not log external request IDs.
- Give admitted output frames a small diagnostic sequence number and admission
  timestamp; record required/optional class, byte count, dequeue/start/end times
  and write success. Pass metadata with the frame rather than reparsing its JSON.
- Record skipped historical eligibility/claim outcomes with compile generation,
  and distinguish serialized, queued, replaced, dequeued and written outcomes.
- Record receiver frame completion and JSON-decode start/end separately in the
  benchmark, preserving original wire bytes and avoiding body logging on stderr.

Keep instrumentation behind diagnostic_timings, maintain existing queue bounds,
source guards and defaults, and test required/optional replacement plus failed
writes. Cross-process timestamps need an explicit shared clock/calibration before
subtracting them; within-process durations alone cannot establish transport time.
Only then should a bounded new capture test which stage dominates. This analysis
runs solely over existing artifacts and makes no additional timing measurement.

## Implemented diagnostic transport probes

With diagnostic_timings enabled, output frames now carry a numeric attempted-
admission sequence, class, byte count and helper-relative admission clock. Stderr
records admission/refusal, optional replacement/reset/required eviction, dequeue,
write start/finish/failure, and a distinct watchdog timeout carrying the active
sequence. Historical frames additionally retain the numeric compile generation,
including when evicted without delivery. Missing binding and eligibility/claim
refusals are explicit scalar outcomes. No raw request IDs, paths or tokens are
included in these new records. Request timing uses a checked numeric sequence and
shares the output clock. Exhausted diagnostic IDs become absent, never wrap.

Queue trace emission occurs after releasing the queue mutex. Diagnostic stderr
must still be actively drained or redirected to a file: these opt-in probes can
perturb scheduling and a blocked diagnostic sink is not made nonblocking. Admission
logging may race dequeue logging, so log-line order is not lifecycle order.
Sequence gaps include refused attempts, and an evicted optional frame never
appears on stdout. Correlate successful writer order and original wire evidence,
not contiguous sequence numbers or byte count alone. A timeout is not a successful
write or proof of partial byte count. Existing watchdog and queue bounds remain.

The benchmark receiver records a Client-wide ordinal including startup frames,
read completion, a separate decode start/end (excluding raw-wire copying), and
frame byte count. Its source-free timing list is capped at4096 records, with a
reported dropped count. Parent receipt handling remains distinct from native paint.
Helper-relative and Python monotonic timestamps are separate clock domains until
explicitly calibrated; compare within-process durations without subtracting origins.

Tests cover replacement/in-flight identity, refused attempts, preserved required
priority, disabled diagnostics, sequence exhaustion, and actual unread-output
watchdog correlation with no false write_finished record. Receiver order and
record bounds were checked without a compiler timing run. Defaults and wire bytes
remain unchanged. A measured diagnostic capture is still a separate future gate.

## Failed instrumented capture retained

`benchmarks/typing-burst-instrumented-failed` records the single approved diagnostic
run with helper e8b5a6fa / binary29810763 and the unchanged release producer1587245d.
The burst and receipt checks reached current21, but the subsequent reopened preview
failed with `compiler response timeout`. Its producer request was captured; the
output contains only an incomplete94,208-byte JSON prefix. Both original producer
PIDs were absent after cleanup. No rerun occurred. GH33 tracks this failure.

Normal successful-run provenance and clean-final direct compile comparison were
not reached. The archive explicitly says failed_post_measurement_reopen and retains
all original bytes, including the partial frame; no complete-run acceptance is
claimed. Source/config timing scripts are hashed separately in failure.json.

The offline transport reviewer checks all numeric frame lifecycles for immutable
metadata, duplicate/contradictory outcomes and missing coverage, then maps writer
order to full-session receiver ordinal, exact length and historical generation.
For the measured prefix, optional writes total17.95ms and receiver JSON decoding
169.44ms. These are separate within-process intervals, not an additive latency
model or a native paint measurement. The reopened timeout cause remains unknown;
these prefix timings do not explain it. The original partial output must be
investigated before calling the full experiment successful.

## Proxy failure propagation and reopen evidence

The original proxy output thread could raise on a producer read, capture write or
forward write while its main thread remained waiting for input. That is a concrete
error-propagation gap, not an established explanation of run68918.

The output pump now reports a scalar stage/error type/errno, attempts a sidecar and
stderr record, and exits126 on an exception; the existing Linux parent-death signal
terminates its producer. Exception message text is omitted. Deterministic tests
cover read failure with process termination, a partial capture-write ENOSPC error
that does not forward the uncaptured frame, forwarding failure after original
capture, and unchanged original bytes including partial EOF. Four tests pass.

Reopen now retains original helper event lines, raw stderr (up to1MiB, explicit
truncation flag) and bounded receiver timing records even when snapshot acceptance
fails. The normal original measurement and failed archive remain unchanged. No
producer/typing rerun was performed for this change.

A later independent Commander decompression under /tmp encountered OSError122
(Disk quota exceeded). That later observation does not establish the earlier
capture's cause; filesystem-wide free space is not quota evidence. Small fault
tests used a temporary directory on /home to avoid repeating that storage failure.

Review added a fifth test for an already-full diagnostic pipe: error logging uses
an independently opened nonblocking Linux pipe descriptor and leaves inherited
flags unchanged. The proxy does not wait for that pipe before exit. Regular-file
and sidecar I/O remain ordinary filesystem operations, not hard real-time bounds.
Reopen capture status explicitly labels its stderr as a pre-stop snapshot; late
shutdown stderr may be missing. Child termination is checked separately from
reaping by the OS, and no remote/native recovery acceptance is implied.

## Positional diagnostic snapshots

The one corrected-harness correctness run under /home (process58242, harness
77c8cab1) failed in Client.diagnostics JSON parsing before reopening. Its original
remaining artifacts are in `typing-burst-diagnostic-read-failed`; raw stderr was
not saved before the temporary file closed, so the exact malformed bytes and
cause are unavailable. No complete acceptance or comparison is claimed.

Code review found a separate concrete defect: seek/read on the diagnostic file
changes the open-file-description offset shared with the helper's stderr writer.
The reader now uses positional pread, retains raw bytes before parsing, and records
explicit partial-tail, truncation and non-JSON states. Only newline-complete records
are parsed; an incomplete final line remains in the raw capture with a flag.
Complete non-JSON records still fail rather than being silently discarded. The
1MiB bound remains. This is a live snapshot, not complete terminal stderr coverage.

Four deterministic tests verify shared-offset preservation and subsequent exact
append, partial-tail completion, preserved/refused non-JSON records, and the capture
bound. They establish the mechanism and repair, not the causes of58242 or68918.
No additional workload was run while making this change.

Snapshot status starts with captured=false until a positional read succeeds. An
earlier failure leaving empty saved bytes therefore does not imply an empty stderr
stream; the status explicitly records that no snapshot was taken.

## Correctness gate passes; full transport trace remains incomplete

`typing-burst-positional-correctness` preserves process77486, the single approved
correctness run after snapshot repair (harness e2072fe3, unchanged29810763 helper
and1587245d producer). All20 edits, current21, five source-bound historical frames,
permanent-ID retries/conflict refusal, exact durable reopen, and clean final direct
compilation passed. Normal original provenance and raw reopen evidence are retained
with archive hashes. Both producer PIDs were absent afterward.

The strict transport reviewer refused this run's complete-trace claim: its live
raw diagnostic snapshot ends mid-record, and a received frame lacks terminal writer
coverage. Status says captured=true, partial_tail=true, truncated=false,
non_json=false. This is explicitly a correctness pass with incomplete transport
attribution, not a benchmark failure hidden as success or a complete performance
measurement. The analyzer was not weakened and no additional run was made.

The next diagnostic improvement is to retain the diagnostic descriptor across
helper termination and snapshot after its writer stops, with bounded cleanup tests.
A previous live snapshot remains separate evidence. Neither this pass nor the
snapshot repair establishes the causes of earlier58242 or68918 failures.
