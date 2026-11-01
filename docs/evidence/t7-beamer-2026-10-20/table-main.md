**NON-REFERENCE run**: power source unknown at start; power source unknown at end; CPU quota of flashtex-agents.slice throttled 36 of 2331 periods (3.6 s stopped); CPU quota of flashtex.slice throttled 2331 of 2331 periods (1333.1 s stopped); load1 up to 9.3 (> 8.0)

| doc | pages | edit | n | ms p50 / p95 / max | gated on | target | first changed page p95 | DONE p95 ms | off-CPU p95 ms | converged | re-typeset pages p50 / max | host RSS peak | load1 | verdict |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| beamer-5 | 5 | letter@start | 19 | 26.1 / 89.3 / 92.7 | p95 | 11 | 89.3 | 216.1 | 65.0 | 19/19 | 2 / 2 | 199 MB | 0.65-0.5 | MISS: p95 89.3 ms > 11 ms (no baseline row) |
| beamer-5 | 5 | letter@middle | 19 | 27.2 / 72.9 / 75.7 | p95 | 11 | 72.9 | 175.2 | 46.0 | 19/19 | 2 / 2 | 199 MB | 0.5-0.46 | MISS: p95 72.9 ms > 11 ms (no baseline row) |
| beamer-5 | 5 | letter@end | 19 | 27.2 / 75.4 / 88.0 | p95 | 11 | 75.4 | 191.8 | 50.1 | 19/19 | 2 / 2 | 199 MB | 0.46-0.39 | MISS: p95 75.4 ms > 11 ms (no baseline row) |
| beamer-5 | 5 | sentence@middle | 19 | 29.1 / 85.0 / 85.4 | p95 | 11 | 85.0 | 196.3 | 52.0 | 19/19 | 2 / 2 | 199 MB | 0.39-0.41 | MISS: p95 85.0 ms > 11 ms (no baseline row) |
| beamer-5 | 5 | newline@middle | 19 | 27.5 / 88.0 / 89.0 | p95 | 11 | 88.0 | 284.4 | 59.5 | 19/19 | 2 / 2 | 199 MB | 0.41-0.35 | MISS: p95 88.0 ms > 11 ms (no baseline row) |
| beamer-5 | 5 | split@middle | 19 | 26.8 / 91.8 / 95.2 | p95 | 11 | 91.8 | 305.0 | 64.1 | 19/19 | 2 / 2 | 199 MB | 0.35-0.29 | MISS: p95 91.8 ms > 11 ms (no baseline row) |
| beamer-5 | 5 | typing@50ms | 39 | 63.7 / 81.9 / 99.9 | p95 | 11 | - | - | 53.0 | - | - | 199 MB | 0.29-0.35 | MISS: p95 81.9 ms > 11 ms; 6 of 39 keystrokes not painted by their own compile |
| beamer-5 | 5 | typing@60ms | 39 | 65.6 / 102.0 / 135.3 | p95 | 11 | - | - | 64.0 | - | - | 199 MB | 0.35-0.35 | MISS: p95 102.0 ms > 11 ms; 4 of 39 keystrokes not painted by their own compile |
| beamer-5 | 5 | typing@80ms | 39 | 52.0 / 95.1 / 106.3 | p95 | 11 | - | - | 52.1 | - | - | 199 MB | 0.35-0.32 | MISS: p95 95.1 ms > 11 ms |
| beamer-5 | 5 | typing@100ms | 39 | 30.7 / 98.9 / 103.3 | p95 | 11 | - | - | 59.0 | - | - | 199 MB | 0.32-0.3 | MISS: p95 98.9 ms > 11 ms |
| beamer-5 | 5 | typing@150ms | 39 | 59.7 / 96.6 / 110.0 | p95 | 11 | - | - | 65.0 | - | - | 199 MB | 0.3-0.27 | MISS: p95 96.6 ms > 11 ms |
| beamer-5 | 5 | preamble | 5 | 1073.6 / 1206.1 / 1206.1 | max | 400 | 1206.1 | 1477.4 | 665.2 | 0/5 | 5 / 5 | 199 MB | 0.27-1.99 | MISS: max 1206.1 ms > 400 ms (no baseline row) |
| beamer-5 | 5 | reopen: host's share, cold (spawn -> page 1) | 3 | 382.3 / 384.2 / 384.2 | max | (100, app) | - | - | - | - | - | 199 MB | 1.99-2.63 | report-only: max 384.2 ms > 100 ms |
| beamer-5 | 5 | reopen: host already listening (not counted, decision 8) | 3 | 206.7 / 213.5 / 213.5 | max | (100, app) | - | - | - | - | - | 199 MB | 1.99-2.63 | report-only: max 213.5 ms > 100 ms |
| beamer-30 | 58 | letter@middle | 19 | 104.3 / 108.7 / 109.5 | p95 | 11 | 108.7 | 289.7 | 52.1 | 19/19 | 2 / 2 | 200 MB | 2.63-4.73 | MISS: p95 108.7 ms > 11 ms (no baseline row) |
| beamer-30 | 58 | letter@end | 19 | 106.0 / 171.9 / 177.4 | p95 | 11 | 171.9 | 369.0 | 116.9 | 19/19 | 2 / 2 | 200 MB | 4.73-5.94 | MISS: p95 171.9 ms > 11 ms (no baseline row) |
| beamer-30 | 58 | sentence@middle | 19 | 120.6 / 181.4 / 186.5 | p95 | 11 | 181.4 | 382.3 | 121.1 | 19/19 | 2 / 2 | 200 MB | 5.94-7.2 | MISS: p95 181.4 ms > 11 ms (no baseline row) |
| beamer-30 | 58 | newline@middle | 19 | 118.8 / 179.0 / 189.0 | p95 | 11 | 179.0 | 481.5 | 120.2 | 19/19 | 3 / 3 | 200 MB | 7.2-8.54 | MISS: p95 179.0 ms > 11 ms (no baseline row) |
| beamer-30 | 58 | split@middle | 19 | 169.1 / 179.5 / 181.8 | p95 | 11 | 179.5 | 503.8 | 121.4 | 19/19 | 3 / 3 | 200 MB | 8.54-9.15 | MISS: p95 179.5 ms > 11 ms (no baseline row) |
| beamer-30 | 58 | letter@start | 19 | 115.3 / 176.5 / 176.6 | p95 | 11 | 176.5 | 375.4 | 116.1 | 19/19 | 2 / 2 | 200 MB | 9.15-9.27 | MISS: p95 176.5 ms > 11 ms (no baseline row) |
| beamer-30 | 58 | typing@50ms | 39 | 87.3 / 255.0 / 274.7 | p95 | 11 | - | - | 52.1 | - | - | 200 MB | 9.27-9.27 | MISS: p95 255.0 ms > 11 ms; 23 of 39 keystrokes not painted by their own compile |
| beamer-30 | 58 | typing@60ms | 39 | 91.3 / 202.0 / 229.3 | p95 | 11 | - | - | 47.2 | - | - | 200 MB | 9.27-9.25 | MISS: p95 202.0 ms > 11 ms; 20 of 39 keystrokes not painted by their own compile |
| beamer-30 | 58 | typing@80ms | 39 | 79.4 / 106.6 / 224.9 | p95 | 11 | - | - | 41.0 | - | - | 200 MB | 9.25-9.15 | MISS: p95 106.6 ms > 11 ms; 2 of 39 keystrokes not painted by their own compile |
| beamer-30 | 58 | typing@100ms | 39 | 89.1 / 100.5 / 103.8 | p95 | 11 | - | - | 47.1 | - | - | 200 MB | 9.15-9.15 | MISS: p95 100.5 ms > 11 ms |
| beamer-30 | 58 | typing@150ms | 39 | 62.8 / 99.1 / 111.6 | p95 | 11 | - | - | 44.3 | - | - | 200 MB | 9.15-9.05 | MISS: p95 99.1 ms > 11 ms |
| beamer-30 | 58 | preamble | 5 | 805.5 / 990.3 / 990.3 | max | 400 | 990.3 | 3449.8 | 452.6 | 0/5 | 58 / 58 | 200 MB | 9.05-8.89 | MISS: max 990.3 ms > 400 ms (no baseline row) |
| beamer-30 | 58 | reopen: host's share, cold (spawn -> page 1) | 3 | 304.9 / 362.8 / 362.8 | max | (100, app) | - | - | - | - | - | 200 MB | 8.9-9.15 | report-only: max 362.8 ms > 100 ms |
| beamer-30 | 58 | reopen: host already listening (not counted, decision 8) | 3 | 183.2 / 213.6 / 213.6 | max | (100, app) | - | - | - | - | - | 200 MB | 8.9-9.15 | report-only: max 213.6 ms > 100 ms |

ms: COMPILE written to the watched page's PAGE frame read on the socket (preamble, reopen: page 1); typing@: the keystroke sent to the watched page from its own compile or a later one. off-CPU: of the host's time to that page, what its engine thread spent descheduled (wall minus queue minus thread CPU): load or a CPU quota, not the engine. The first sample of each phase is a warm-up, left out; rows with n < 12 are gated on their maximum. No noise margin: a row passes only when it meets its target.
