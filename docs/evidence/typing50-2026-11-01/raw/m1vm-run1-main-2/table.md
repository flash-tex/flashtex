**NON-REFERENCE run**: load1 up to 17.1 (> 1.5)

| doc | pages | edit | n | ms p50 / p95 / max | gated on | target | first changed page p95 | DONE p95 ms | off-CPU p95 ms | converged | re-typeset pages p50 / max | host RSS peak | load1 | verdict |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| infdesc-x2 | 1152 | letter@x2-late | 7 | 79.5 / 125.8 / 125.8 | max | 20 | 125.8 | 671.9 | 14.8 | 7/7 | 2 / 2 | 1113 MB | 17.05-4.65 | PASS (no instruction count on this machine (stages.edited_instr_k): not gated) |
| infdesc-x2 | 1152 | typing@50ms@x2-late | 15 | 205.0 / 502.5 / 502.5 | p95 | 20 | - | - | 2.6 | - | - | 1113 MB | 4.65-3.83 | MISS: 13 of 15 keystrokes not painted by their own compile |
| infdesc-x2 | 1152 | typing@150ms@x2-late | 15 | 70.7 / 171.4 / 192.2 | p95 | 20 | - | - | 17.5 | - | - | 1113 MB | 3.83-3.54 | MISS: 2 of 15 keystrokes not painted by their own compile |
| infdesc-x2 | 1152 | typing@100ms@x2-late | 15 | 62.2 / 156.8 / 186.5 | p95 | 20 | - | - | 14.9 | - | - | 1113 MB | 3.54-3.04 | MISS: 6 of 15 keystrokes not painted by their own compile |

ms: COMPILE written to the watched page's PAGE frame read on the socket (preamble, reopen: page 1; open: page 1, every page of the final count, DONE with the tools settled); typing@: the keystroke sent to the watched page from its own compile or a later one. off-CPU: of the host's time to that page, what its engine thread spent descheduled (wall minus queue minus thread CPU): load or a CPU quota, not the engine. The first sample of each phase is a warm-up, left out; rows with n < 12 are gated on their maximum. No noise margin: a row passes only when it meets its target.
