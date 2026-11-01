**NON-REFERENCE run**: power source unknown at start; power source unknown at end

| doc | pages | edit | n | ms p50 / p95 / max | gated on | target | first changed page p95 | DONE p95 ms | off-CPU p95 ms | converged | re-typeset pages p50 / max | host RSS peak | load1 | verdict |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| plain-10 | 10 | letter@start | 19 | 6.2 / 11.4 / 11.6 | p95 | 11 | 11.4 | 32.4 | 0.1 | 19/19 | 2 / 2 | 142 MB | 0.41-0.37 | MISS: p95 11.4 ms > 11 ms |
| plain-10 | 10 | letter@middle | 19 | 5.0 / 5.5 / 6.0 | p95 | 11 | 2.7 | 25.5 | 0.0 | 19/19 | 3 / 3 | 142 MB | 0.37-0.42 | PASS |
| full-10 | 11 | letter@start | 19 | 13.6 / 19.8 / 20.1 | p95 | 11 | 19.8 | 57.4 | 0.3 | 19/19 | 2 / 2 | 163 MB | 0.42-0.43 | MISS: p95 19.8 ms > 11 ms |
| full-10 | 11 | letter@middle | 19 | 9.3 / 12.7 / 16.6 | p95 | 11 | 6.2 | 49.8 | 0.0 | 19/19 | 3 / 3 | 163 MB | 0.43-0.4 | MISS: p95 12.7 ms > 11 ms |

ms: COMPILE written to the watched page's PAGE frame read on the socket (preamble, reopen: page 1); typing@: the keystroke sent to the watched page from its own compile or a later one. off-CPU: of the host's time to that page, what its engine thread spent descheduled (wall minus queue minus thread CPU): load or a CPU quota, not the engine. The first sample of each phase is a warm-up, left out; rows with n < 12 are gated on their maximum. No noise margin: a row passes only when it meets its target.
