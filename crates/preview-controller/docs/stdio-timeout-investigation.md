# GH33: large-preview test observation and reader lifetime

The initial FT048r12 full stdio run passed28/30 cases. The configured large
original-compiler preview timed out after10seconds; optional expansion did not
report serialization refusal before its15second observation deadline. Both exact
unchanged isolated reruns passed (2.01s and0.79s). A concurrent integration build
was present on the first run. This is consistent with contention, but does not
establish the original cause or rule out a timing-sensitive product defect.

The large preview deadline starts after the helper ready frame and includes the
compiler, runtime parsing, helper emission, pipe transfer and test reader's JSON
parse before its channel delivery. It is not a helper-only latency measurement.
The expansion test includes Python fixture creation, runtime parsing, binding and
optional offer. Its last compiler_poll diagnostic cannot identify all missing
phases; absence of a refusal line is not proof serialization itself hung.

The test Client previously discarded its reader JoinHandle. Killing/reaping the
helper did not prove that the reader had finished decoding an already received
large frame. This allowed potential reader work to overlap the next test, though
no retained evidence proves that happened in the original failure.

The harness now keeps the reader handle, checks terminal state for up to3seconds
after helper kill/reap, and joins only once terminal. Nontermination fails cleanup;
unwinding logs it instead of causing a second panic. Deliberately unread-output
fixtures have no reader and retain their existing watchdog behavior. The3second
cleanup bound does not extend the10/15second acceptance deadlines.

Reader snapshots contain only phase, decoded frame count, last frame byte length
and last completed decode duration. Timeout messages include those scalars and at
most8KiB of existing helper phase diagnostics. Snapshots release the mutex before
panic formatting. No source text is added to diagnostics; no production output
limits, watchdogs or scheduling policies change. The actual compiler fixture now
enables the existing optional diagnostic timings for failure attribution.

GH33 remains an unresolved-cause investigation. Clean reruns or this harness
change alone do not establish a product fix or a calibrated responsiveness result.
