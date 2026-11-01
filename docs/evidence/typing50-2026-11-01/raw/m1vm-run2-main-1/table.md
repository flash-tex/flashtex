**NON-REFERENCE run**: load1 up to 14.7 (> 1.5)

| doc | pages | edit | n | ms p50 / p95 / max | gated on | target | first changed page p95 | DONE p95 ms | off-CPU p95 ms | converged | re-typeset pages p50 / max | host RSS peak | load1 | verdict |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| infdesc-x2 | 1152 | letter@x2-late | 7 | 62.7 / 102.5 / 102.5 | max | 20 | 102.5 | 642.7 | 11.3 | 7/7 | 2 / 2 | 1023 MB | 12.62-13.57 | PASS (no instruction count on this machine (stages.edited_instr_k): not gated) |
| infdesc-x2 | 1152 | typing@50ms@x2-late | 15 | 561.0 / 851.7 / 851.8 | p95 | 20 | - | - | 0.5 | - | - | 1023 MB | 13.57-14.68 | MISS: 14 of 15 keystrokes not painted by their own compile |
| infdesc-x2 | 1152 | typing@150ms@x2-late | 15 | 83.8 / 181.1 / 185.6 | p95 | 20 | - | - | 57.5 | - | - | 1023 MB | 14.68-13.59 | MISS: 1 of 15 keystrokes not painted by their own compile |
| infdesc-x2 | 1152 | typing@100ms@x2-late | 15 | 93.4 / 117.5 / 269.4 | p95 | 20 | - | - | 21.9 | - | - | 1023 MB | 13.59-13.59 | MISS: 5 of 15 keystrokes not painted by their own compile |

ms: COMPILE written to the watched page's PAGE frame read on the socket (preamble, reopen: page 1; open: page 1, every page of the final count, DONE with the tools settled); typing@: the keystroke sent to the watched page from its own compile or a later one. off-CPU: of the host's time to that page, what its engine thread spent descheduled (wall minus queue minus thread CPU): load or a CPU quota, not the engine. The first sample of each phase is a warm-up, left out; rows with n < 12 are gated on their maximum. No noise margin: a row passes only when it meets its target.
