**NON-REFERENCE run**: load1 up to 10.7 (> 1.5)

| doc | pages | edit | n | ms p50 / p95 / max | gated on | target | first changed page p95 | DONE p95 ms | off-CPU p95 ms | converged | re-typeset pages p50 / max | host RSS peak | load1 | verdict |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| infdesc-x2 | 1152 | letter@x2-late | 7 | 64.4 / 87.8 / 87.8 | max | 20 | 87.8 | 485.6 | 36.8 | 7/7 | 2 / 2 | 1112 MB | 10.69-6.75 | PASS (no instruction count on this machine (stages.edited_instr_k): not gated) |
| infdesc-x2 | 1152 | typing@50ms@x2-late | 15 | 80.6 / 99.0 / 110.6 | p95 | 20 | - | - | 19.7 | - | - | 1112 MB | 6.75-6.29 | MISS: 7 of 15 keystrokes not painted by their own compile |
| infdesc-x2 | 1152 | typing@150ms@x2-late | 15 | 77.2 / 148.1 / 270.0 | p95 | 20 | - | - | 21.5 | - | - | 1112 MB | 6.29-6.29 | PASS |
| infdesc-x2 | 1152 | typing@100ms@x2-late | 15 | 85.7 / 180.2 / 242.5 | p95 | 20 | - | - | 26.2 | - | - | 1112 MB | 6.29-6.29 | MISS: 3 of 15 keystrokes not painted by their own compile |

ms: COMPILE written to the watched page's PAGE frame read on the socket (preamble, reopen: page 1; open: page 1, every page of the final count, DONE with the tools settled); typing@: the keystroke sent to the watched page from its own compile or a later one. off-CPU: of the host's time to that page, what its engine thread spent descheduled (wall minus queue minus thread CPU): load or a CPU quota, not the engine. The first sample of each phase is a warm-up, left out; rows with n < 12 are gated on their maximum. No noise margin: a row passes only when it meets its target.
