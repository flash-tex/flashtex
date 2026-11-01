# Compiler stops reading stdin

The Linux acceptance test establishes a blocked write without sleep-based
readiness. A real child reads exactly one stdin byte, reports the pipe capacity
through a Unix datagram, then never reads another byte. The admitted request is
8,388,261 bytes against a measured 65,536-byte pipe, within the unchanged 8-MiB
frame limit. A second project remains queued.

Three modes pass: deterministic active-deadline expiry, project close followed
by session drop, and direct session drop. Each kills and reaps the direct child,
then observes a separate test-only writer-exit notification. No request becomes
a preview. Project close alone intentionally preserves the shared session and
marks the active request cancelled; session drop performs kill/reap. The timeout
case sets the private dispatch timestamp into the past, so it proves the timeout
transition rather than a scheduler-dependent wait duration.

Observed failure/drop completion was 0.85/0.48/0.44 ms respectively; writer terminal
was witnessed by 0.87/0.50/0.46 ms. These are single Linux debug-test observations,
not worst-case deadlines. Complete logs and hashes are in `../benchmarks/blocked-writer`.
The writer-exit witness exists only in Linux test builds. No production change,
thread, queue, or cap was introduced.

The decoder is explicitly joined. Writer/stdout/stderr threads currently have
detached handles; the test witnesses writer completion separately and does not
claim an all-thread join. It covers a direct child that owns its stdin reader,
not arbitrary descendants inheriting pipe descriptors. No lifecycle defect was
demonstrated in these cases, so no speculative shutdown refactor is included.

Reproduce with `cargo test --lib blocked_writer -- --ignored --nocapture` using
the document-runtime manifest. The test requires Linux and `/usr/bin/python3`,
and does not run an external compiler or require font assets.

## Repeated session acceptance

The follow-up test runs 30 sequential sessions: 10 normal result/drop, 10
stopped-reader timeout and 10 stopped-reader close/drop, split evenly between
Value and raw constructors. Normal requests must produce a preview; stopped
requests must not. The same one-byte handshake and near-limit admitted request
profile establish blocked writes. Test-only stdout/stderr exit notifications
complement the existing writer witness; the decoder remains explicitly joined.

After all three I/O witnesses, exact child PID reaping and local handshake cleanup,
a deadline-bounded condition loop checks `/proc/self/fd`, `/proc/self/task` and
`/proc/thread-self/children`. The loop permits the small thread epilogue following
an exit notification; it does not use a sleep as readiness evidence. All 30 cycles
returned to the observed baseline: 4 descriptors, 2 threads, no children. Exact
per-cycle PID/counters are preserved in `../benchmarks/session-cycles`.

No accumulation was demonstrated in these direct-child cases. OS counters are
not allocator/RSS evidence and do not establish production all-thread joining or
a universal shutdown deadline. No production refactor is included. Run the ignored
unit test `blocked_writer::repeated_sessions_return_to_os_baseline_after_worker_witnesses`
alone, optionally setting `FLASHTEX_CYCLE_EVIDENCE` to an output JSON path.
