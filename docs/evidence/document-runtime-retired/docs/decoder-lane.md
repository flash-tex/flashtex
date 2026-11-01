# Bounded compiler JSON decoder

Compiler stdout still enters the existing four-message raw queue with the same
configured frame limit. One worker validates UTF-8 and decodes JSON. A single
permit covers parsing, the one decoded queue slot, and the owner-held packet
through semantic/source validation. The worker cannot begin decoding the next
raw frame until that packet is released. No queue of expanded Values is added.
Raw bytes are dropped after decoding; the existing parser expansion and temporary
raw-plus-Value overlap remain, relocated from owner to worker. One OS-thread stack
is additional. This is not a process-RSS bound.

The serialized Session owner retains exact active request/source/capability checks.
Malformed, cancelled and stale results still pass the same required validation;
a decoded Value never grants preview or source-action authority. Packet RAII
releases the permit on every successful, refusal, stale and cancelled exit.
The decoder itself knows no current revision or source token and cannot rebind
an old reply to new work.

Shutdown sets a stop flag, drops the output receiver, wakes permit waiting and
joins the worker. Raw-input waiting checks shutdown every10ms; holding a packet
outside the decoder cannot deadlock shutdown. A current bounded-frame serde parse
is allowed to finish: cancellation is not instantaneous. Process teardown still
kills/reaps the original compiler. Existing IO-thread behavior is unchanged.

ResponseProfile adds decode_queue_wait_ms (raw frame awaiting decoding), while
reader_delivery_wait_ms now measures decoded completion awaiting owner polling.
parse_ms is worker parse time, validation_ms is owner semantic validation. Neither
these phases nor helper timings measure native paint.

31runtime tests (including the explicit original compiler gate) and53serial helper
tests pass with strict all-target lint. Five new deterministic tests cover one
permit through owner validation, four raw slots, malformed/invalidUTF8 rejection,
shutdown with caller-held packet, idle raw producer, and both queues full. Existing
source/cancel/stale/restart/malformed/flood and exact original-output checks remain.
