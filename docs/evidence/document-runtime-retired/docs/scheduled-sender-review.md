# Scheduled sender: independent lifecycle review

FT049r27; read-only review of root helper candidate on2026-09-12. No peer edits,
workload rerun, sender execution or production runtime change was performed.

Reviewed candidate SHA256 values (uncommitted candidate, not a publication claim):

- `scheduled_sender.py`: `d3d088ce1663a0f7daaedf306341d378b8294ae399b57caed2bef714370f8590`
- `typing_burst.py`: `81d3359b43d56485da12d1eda1f8418502714ee81810eb9a68b2fc09c09771ff`
- `test_scheduled_sender.py`: `491dfa08856ffeb7b593067f0ade20d10f868fad21b0e6f76101c782cd3cdd48`

The parent pre-serializes exact request frames, flushes its buffered stdin handle,
and passes the descriptor exclusively to one child during the burst. Base64 config
transport preserves bytes. The child uses at-most-PIPE_BUF writes after writable
select, preserving shared descriptor flags and frame order under the sole-writer
assumption. Parent finish reaps the child before retry/configuration writes. Actual
send timestamps remain required; a200ms startup lead does not guarantee cadence.

Two concrete error paths were sent to the helper owner before publication:

1. Sender `finish()` runs only after all ACKs/final preview. A failed sender can thus
   be masked by a later receiver timeout. Check child failure before/after bounded
   reads and preserve the sender's error evidence on that failure path.
2. Sequential `sender.stop(); client.stop()` in `finally` skips helper cleanup if
   sender stop raises. Nest cleanup with `finally` so helper stop and event-file
   close remain attempted independently.

The owner reported five lifecycle tests passing; this reviewer inspected their
coverage, did not independently execute them, and does not claim broader coverage.
Explicit cancellation/reaping differs from abrupt parent death: no parent-death
signal was present. A finite child deadline limits continued sending in that case,
but does not prove immediate parent-death termination or an elapsed-time guarantee.

This document records the reviewed candidate and requested changes. It does not
assert that later owner fixes have been reviewed or authorize a throughput run.

## Published correction reviewed

Read-only review of `dfa69e19` confirms both requested corrections: checks before,
after and on exceptions from receiver reads surface a recorded sender failure;
nested cleanup attempts helper stop and event close even if sender stop raises.
Config/result evidence is retained under output `sender/`, including failure cases.
No independent executable test or measured burst was run by this reviewer.

Failure does not interrupt a `Client.read()` already blocked in its existing15s
select deadline. The saved child error is surfaced when that bounded read returns
or times out. This is eventual error reporting, not immediate child-death wakeup.
Successful finish reaps before parent writes; child closes only its inherited pipe
handle, leaving the parent descriptor available. Parent flush before spawn avoids
buffered interleaving under the exclusive-writer assumption.

The existing `Client.stop()` closes descriptors only after successful kill/wait;
a helper reap failure can therefore skip those closes, though the new outer cleanup
still closes events. No guarantee that every resource closes under every OS failure,
or that abrupt parent death immediately terminates the sender, is asserted. These
limits do not invalidate the captured source/result audit or create a new runtime
production requirement.
