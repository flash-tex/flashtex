**NON-REFERENCE run**: power source unknown at start; power source unknown at end

| doc | pages | edit | n | ms p50 / p95 / max | gated on | target | first changed page p95 | DONE p95 ms | off-CPU p95 ms | converged | re-typeset pages p50 / max | host RSS peak | load1 | verdict |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| plain-10 | 10 | letter@start | 19 | 6.4 / 10.5 / 10.8 | p95 | 11 | 10.5 | 67.3 | 0.1 | 0/19 | 10 / 10 | 141 MB | 0.18-0.3 | MISS: convergence 0.00 < baseline 1.00 |
| plain-10 | 10 | letter@middle | 19 | 4.8 / 5.7 / 6.2 | p95 | 11 | 2.7 | 27.6 | 0.0 | 19/19 | 3 / 3 | 141 MB | 0.3-0.28 | PASS |
| full-10 | 11 | letter@start | 19 | 13.7 / 20.0 / 20.6 | p95 | 11 | 20.0 | 57.8 | 0.3 | 19/19 | 2 / 2 | 161 MB | 0.28-0.38 | MISS: p95 20.0 ms > 11 ms |
| full-10 | 11 | letter@middle | 19 | 9.2 / 10.3 / 10.7 | p95 | 11 | 4.4 | 46.3 | 0.0 | 19/19 | 3 / 3 | 161 MB | 0.38-0.35 | PASS |

ms: COMPILE written to the watched page's PAGE frame read on the socket (preamble, reopen: page 1); typing@: the keystroke sent to the watched page from its own compile or a later one. off-CPU: of the host's time to that page, what its engine thread spent descheduled (wall minus queue minus thread CPU): load or a CPU quota, not the engine. The first sample of each phase is a warm-up, left out; rows with n < 12 are gated on their maximum. No noise margin: a row passes only when it meets its target.
