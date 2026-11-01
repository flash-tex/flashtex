**NON-REFERENCE run**: power source unknown at start; power source unknown at end; CPU quota of flashtex-agents.slice throttled 1416 of 1646 periods (168.3 s stopped)

| doc | pages | edit | n | ms p50 / p95 / max | gated on | target | first changed page p95 | DONE p95 ms | off-CPU p95 ms | converged | re-typeset pages p50 / max | host RSS peak | load1 | verdict |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| beamer-5 | 5 | letter@start | 19 | 5.0 / 6.3 / 6.6 | p95 | 11 | 6.3 | 40.5 | 0.0 | 19/19 | 2 / 2 | 231 MB | 1.57-1.48 | PASS (no baseline row) |
| beamer-5 | 5 | letter@middle | 19 | 6.1 / 6.4 / 12.4 | p95 | 11 | 6.4 | 75.1 | 0.0 | 19/19 | 2 / 2 | 231 MB | 1.48-1.37 | PASS (no baseline row) |
| beamer-5 | 5 | letter@end | 19 | 5.9 / 6.1 / 6.2 | p95 | 11 | 6.1 | 74.4 | 0.0 | 19/19 | 2 / 2 | 231 MB | 1.37-1.16 | PASS (no baseline row) |
| beamer-5 | 5 | sentence@middle | 19 | 5.9 / 6.8 / 7.5 | p95 | 11 | 6.8 | 62.9 | 0.0 | 19/19 | 2 / 2 | 231 MB | 1.16-1.06 | PASS (no baseline row) |
| beamer-5 | 5 | newline@middle | 19 | 6.2 / 6.8 / 31.4 | p95 | 11 | 6.8 | 71.8 | 0.0 | 19/19 | 2 / 3 | 231 MB | 1.06-0.98 | PASS (no baseline row) |
| beamer-5 | 5 | split@middle | 19 | 6.0 / 6.5 / 37.3 | p95 | 11 | 6.5 | 101.4 | 0.1 | 19/19 | 3 / 3 | 231 MB | 0.98-1.6 | PASS (no baseline row) |
| beamer-5 | 5 | typing@50ms | 39 | 7.8 / 30.7 / 31.2 | p95 | 11 | - | - | 24.1 | - | - | 231 MB | 1.6-1.6 | MISS: p95 30.7 ms > 11 ms |
| beamer-5 | 5 | typing@60ms | 39 | 5.0 / 23.0 / 26.3 | p95 | 11 | - | - | 0.4 | - | - | 231 MB | 1.6-1.79 | MISS: p95 23.0 ms > 11 ms |
| beamer-5 | 5 | typing@80ms | 39 | 5.3 / 26.2 / 28.9 | p95 | 11 | - | - | 0.0 | - | - | 231 MB | 1.79-1.79 | MISS: p95 26.2 ms > 11 ms |
| beamer-5 | 5 | typing@100ms | 39 | 5.1 / 5.7 / 6.5 | p95 | 11 | - | - | 0.0 | - | - | 231 MB | 1.79-2.13 | PASS |
| beamer-5 | 5 | typing@150ms | 39 | 5.6 / 6.3 / 7.1 | p95 | 11 | - | - | 0.0 | - | - | 231 MB | 2.13-2.36 | PASS |
| beamer-5 | 5 | preamble | 5 | 603.4 / 664.7 / 664.7 | max | 400 | 664.7 | 716.3 | 270.3 | 0/5 | 5 / 5 | 231 MB | 2.36-2.77 | MISS: max 664.7 ms > 400 ms (no baseline row) |
| beamer-5 | 5 | reopen: host's share, cold (spawn -> page 1) | 3 | 189.2 / 194.8 / 194.8 | max | (100, app) | - | - | - | - | - | 231 MB | 2.77-2.77 | report-only: max 194.8 ms > 100 ms |
| beamer-5 | 5 | reopen: host already listening (not counted, decision 8) | 3 | 102.8 / 106.7 / 106.7 | max | (100, app) | - | - | - | - | - | 231 MB | 2.77-2.77 | report-only: max 106.7 ms > 100 ms |
| beamer-30 | 58 | letter@middle | 19 | 14.8 / 46.6 / 47.2 | p95 | 11 | 46.6 | 97.8 | 31.2 | 19/19 | 2 / 2 | 290 MB | 2.77-2.88 | MISS: p95 46.6 ms > 11 ms (no baseline row) |
| beamer-30 | 58 | letter@end | 19 | 13.8 / 21.8 / 28.1 | p95 | 11 | 21.8 | 88.9 | 6.7 | 19/19 | 2 / 2 | 290 MB | 2.88-3.06 | MISS: p95 21.8 ms > 11 ms (no baseline row) |
| beamer-30 | 58 | sentence@middle | 19 | 14.0 / 29.0 / 31.9 | p95 | 11 | 29.0 | 84.5 | 15.2 | 19/19 | 2 / 2 | 290 MB | 3.06-3.05 | MISS: p95 29.0 ms > 11 ms (no baseline row) |
| beamer-30 | 58 | newline@middle | 19 | 13.7 / 14.6 / 15.3 | p95 | 11 | 14.6 | 140.0 | 0.1 | 19/19 | 3 / 5 | 290 MB | 3.05-3.05 | MISS: p95 14.6 ms > 11 ms (no baseline row) |
| beamer-30 | 58 | split@middle | 19 | 13.9 / 15.3 / 32.9 | p95 | 11 | 15.3 | 184.5 | 0.1 | 19/19 | 5 / 5 | 290 MB | 3.05-3.04 | MISS: p95 15.3 ms > 11 ms (no baseline row) |
| beamer-30 | 58 | letter@start | 19 | 17.2 / 36.6 / 36.7 | p95 | 11 | 36.6 | 90.4 | 19.2 | 19/19 | 2 / 2 | 290 MB | 3.04-3.04 | MISS: p95 36.6 ms > 11 ms (no baseline row) |
| beamer-30 | 58 | typing@50ms | 39 | 32.4 / 39.9 / 54.8 | p95 | 11 | - | - | 26.2 | - | - | 290 MB | 3.04-3.04 | MISS: p95 39.9 ms > 11 ms; 1 of 39 keystrokes not painted by their own compile |
| beamer-30 | 58 | typing@60ms | 39 | 18.1 / 38.9 / 39.9 | p95 | 11 | - | - | 25.2 | - | - | 290 MB | 3.04-3.11 | MISS: p95 38.9 ms > 11 ms |
| beamer-30 | 58 | typing@80ms | 39 | 13.2 / 37.3 / 37.9 | p95 | 11 | - | - | 22.7 | - | - | 290 MB | 3.11-3.26 | MISS: p95 37.3 ms > 11 ms |
| beamer-30 | 58 | typing@100ms | 39 | 13.1 / 13.8 / 15.2 | p95 | 11 | - | - | 0.1 | - | - | 290 MB | 3.26-3.26 | MISS: p95 13.8 ms > 11 ms |
| beamer-30 | 58 | typing@150ms | 39 | 25.2 / 30.1 / 31.2 | p95 | 11 | - | - | 16.2 | - | - | 290 MB | 3.26-3.38 | MISS: p95 30.1 ms > 11 ms |
| beamer-30 | 58 | preamble | 5 | 538.8 / 569.8 / 569.8 | max | 400 | 569.8 | 2328.4 | 140.2 | 0/5 | 58 / 58 | 290 MB | 3.38-3.16 | MISS: max 569.8 ms > 400 ms (no baseline row) |
| beamer-30 | 58 | reopen: host's share, cold (spawn -> page 1) | 3 | 152.0 / 155.2 / 155.2 | max | (100, app) | - | - | - | - | - | 290 MB | 2.91-2.99 | report-only: max 155.2 ms > 100 ms |
| beamer-30 | 58 | reopen: host already listening (not counted, decision 8) | 3 | 92.2 / 98.1 / 98.1 | max | (100, app) | - | - | - | - | - | 290 MB | 2.91-2.99 | report-only |

ms: COMPILE written to the watched page's PAGE frame read on the socket (preamble, reopen: page 1); typing@: the keystroke sent to the watched page from its own compile or a later one. off-CPU: of the host's time to that page, what its engine thread spent descheduled (wall minus queue minus thread CPU): load or a CPU quota, not the engine. The first sample of each phase is a warm-up, left out; rows with n < 12 are gated on their maximum. No noise margin: a row passes only when it meets its target.
