**NON-REFERENCE run**: load1 up to 12.9 (> 1.5)

| doc | pages | edit | n | ms p50 / p95 / max | gated on | target | first changed page p95 | DONE p95 ms | off-CPU p95 ms | converged | re-typeset pages p50 / max | host RSS peak | load1 | verdict |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| infdesc-x2 | 1152 | letter@x2-late | 7 | 38.1 / 54.9 / 54.9 | max | 20 | 54.9 | 379.0 | 6.6 | 7/7 | 2 / 2 | 984 MB | 12.9-8.77 | PASS (no instruction count on this machine (stages.edited_instr_k): not gated) |
| infdesc-x2 | 1152 | typing@50ms@x2-late | 15 | 53.2 / 69.7 / 116.8 | p95 | 20 | - | - | 12.8 | - | - | 984 MB | 8.77-8.77 | MISS: 8 of 15 keystrokes not painted by their own compile |
| infdesc-x2 | 1152 | typing@150ms@x2-late | 15 | 56.5 / 77.3 / 89.5 | p95 | 20 | - | - | 13.1 | - | - | 984 MB | 8.77-8.23 | PASS |
| infdesc-x2 | 1152 | typing@100ms@x2-late | 15 | 56.4 / 93.3 / 107.3 | p95 | 20 | - | - | 16.4 | - | - | 984 MB | 8.23-8.23 | MISS: 5 of 15 keystrokes not painted by their own compile |

ms: COMPILE written to the watched page's PAGE frame read on the socket (preamble, reopen: page 1; open: page 1, every page of the final count, DONE with the tools settled); typing@: the keystroke sent to the watched page from its own compile or a later one. off-CPU: of the host's time to that page, what its engine thread spent descheduled (wall minus queue minus thread CPU): load or a CPU quota, not the engine. The first sample of each phase is a warm-up, left out; rows with n < 12 are gated on their maximum. No noise margin: a row passes only when it meets its target.
