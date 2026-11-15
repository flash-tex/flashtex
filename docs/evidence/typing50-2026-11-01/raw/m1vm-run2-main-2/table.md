**NON-REFERENCE run**: load1 up to 12.6 (> 1.5)

| doc | pages | edit | n | ms p50 / p95 / max | gated on | target | first changed page p95 | DONE p95 ms | off-CPU p95 ms | converged | re-typeset pages p50 / max | host RSS peak | load1 | verdict |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| infdesc-x2 | 1152 | letter@x2-late | 7 | 64.0 / 87.7 / 87.7 | max | 20 | 87.7 | 495.6 | 12.8 | 7/7 | 2 / 2 | 1091 MB | 12.58-4.16 | PASS (no instruction count on this machine (stages.edited_instr_k): not gated) |
| infdesc-x2 | 1152 | typing@50ms@x2-late | 15 | 250.4 / 523.0 / 523.1 | p95 | 20 | - | - | 19.4 | - | - | 1091 MB | 4.16-3.67 | MISS: 11 of 15 keystrokes not painted by their own compile |
| infdesc-x2 | 1152 | typing@150ms@x2-late | 15 | 85.9 / 128.2 / 134.0 | p95 | 20 | - | - | 22.4 | - | - | 1091 MB | 3.67-3.46 | MISS: 5 of 15 keystrokes not painted by their own compile |
| infdesc-x2 | 1152 | typing@100ms@x2-late | 15 | 68.2 / 159.2 / 166.1 | p95 | 20 | - | - | 21.9 | - | - | 1091 MB | 3.46-3.05 | MISS: 5 of 15 keystrokes not painted by their own compile |

ms: COMPILE written to the watched page's PAGE frame read on the socket (preamble, reopen: page 1; open: page 1, every page of the final count, DONE with the tools settled); typing@: the keystroke sent to the watched page from its own compile or a later one. off-CPU: of the host's time to that page, what its engine thread spent descheduled (wall minus queue minus thread CPU): load or a CPU quota, not the engine. The first sample of each phase is a warm-up, left out; rows with n < 12 are gated on their maximum. No noise margin: a row passes only when it meets its target.
