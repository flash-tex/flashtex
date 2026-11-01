**NON-REFERENCE run**: load1 up to 17.1 (> 1.5)

| doc | pages | edit | n | ms p50 / p95 / max | gated on | target | first changed page p95 | DONE p95 ms | off-CPU p95 ms | converged | re-typeset pages p50 / max | host RSS peak | load1 | verdict |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| infdesc-x2 | 1152 | letter@x2-late | 7 | 68.8 / 98.1 / 98.1 | max | 20 | 98.1 | 607.2 | 12.3 | 7/7 | 2 / 2 | 1024 MB | 10.27-14.84 | PASS (no instruction count on this machine (stages.edited_instr_k): not gated) |
| infdesc-x2 | 1152 | typing@50ms@x2-late | 15 | 88.3 / 440.5 / 440.6 | p95 | 20 | - | - | 20.0 | - | - | 1024 MB | 14.84-15.81 | MISS: 9 of 15 keystrokes not painted by their own compile |
| infdesc-x2 | 1152 | typing@150ms@x2-late | 15 | 181.5 / 547.8 / 569.1 | p95 | 20 | - | - | 19.4 | - | - | 1024 MB | 15.81-17.06 | MISS: 7 of 15 keystrokes not painted by their own compile |
| infdesc-x2 | 1152 | typing@100ms@x2-late | 15 | 115.3 / 246.0 / 246.0 | max | 20 | - | - | 38.9 | - | - | 1024 MB | 17.06-17.06 | MISS: 6 of 15 without the watched page; 4 of 15 keystrokes not painted by their own compile |

ms: COMPILE written to the watched page's PAGE frame read on the socket (preamble, reopen: page 1; open: page 1, every page of the final count, DONE with the tools settled); typing@: the keystroke sent to the watched page from its own compile or a later one. off-CPU: of the host's time to that page, what its engine thread spent descheduled (wall minus queue minus thread CPU): load or a CPU quota, not the engine. The first sample of each phase is a warm-up, left out; rows with n < 12 are gated on their maximum. No noise margin: a row passes only when it meets its target.
