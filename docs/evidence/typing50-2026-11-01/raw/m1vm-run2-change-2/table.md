**NON-REFERENCE run**: load1 up to 14.7 (> 1.5)

| doc | pages | edit | n | ms p50 / p95 / max | gated on | target | first changed page p95 | DONE p95 ms | off-CPU p95 ms | converged | re-typeset pages p50 / max | host RSS peak | load1 | verdict |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| infdesc-x2 | 1152 | letter@x2-late | 7 | 38.2 / 67.2 / 67.2 | max | 20 | 67.2 | 384.0 | 28.7 | 7/7 | 2 / 2 | 1021 MB | 8.23-14.71 | PASS (no instruction count on this machine (stages.edited_instr_k): not gated) |
| infdesc-x2 | 1152 | typing@50ms@x2-late | 15 | 78.0 / 129.4 / 133.1 | p95 | 20 | - | - | 74.9 | - | - | 1021 MB | 14.71-14.71 | MISS: 7 of 15 keystrokes not painted by their own compile |
| infdesc-x2 | 1152 | typing@150ms@x2-late | 15 | 56.6 / 92.1 / 98.1 | p95 | 20 | - | - | 24.2 | - | - | 1021 MB | 14.71-14.71 | MISS: 1 of 15 keystrokes not painted by their own compile |
| infdesc-x2 | 1152 | typing@100ms@x2-late | 15 | 54.7 / 85.7 / 85.7 | p95 | 20 | - | - | 13.1 | - | - | 1021 MB | 14.71-13.69 | MISS: 4 of 15 keystrokes not painted by their own compile |

ms: COMPILE written to the watched page's PAGE frame read on the socket (preamble, reopen: page 1; open: page 1, every page of the final count, DONE with the tools settled); typing@: the keystroke sent to the watched page from its own compile or a later one. off-CPU: of the host's time to that page, what its engine thread spent descheduled (wall minus queue minus thread CPU): load or a CPU quota, not the engine. The first sample of each phase is a warm-up, left out; rows with n < 12 are gated on their maximum. No noise margin: a row passes only when it meets its target.
