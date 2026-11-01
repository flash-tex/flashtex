**NON-REFERENCE run**: load1 up to 7.3 (> 4.0)

| doc | pages | edit | n | ms p50 / p95 / max | gated on | target | first changed page p95 | DONE p95 ms | off-CPU p95 ms | converged | re-typeset pages p50 / max | host RSS peak | load1 | verdict |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| plain-10 | 10 | letter@start | 19 | 7.0 / 11.6 / 11.9 | p95 | 11 | 11.6 | 53.5 | 0.5 | 0/19 | 10 / 10 | 154 MB | 3.02-3.02 | MISS: p95 11.6 ms > 11 ms; convergence 0.00 < baseline 1.00 |
| plain-10 | 10 | letter@middle | 19 | 4.4 / 5.1 / 5.2 | p95 | 11 | 3.3 | 21.2 | 0.4 | 19/19 | 3 / 3 | 154 MB | 3.02-3.38 | PASS |
| plain-10 | 10 | letter@end | 19 | 3.3 / 3.5 / 3.5 | p95 | 11 | 3.5 | 11.4 | 0.2 | 0/19 | 1 / 1 | 154 MB | 3.38-3.59 | PASS |
| plain-10 | 10 | sentence@middle | 19 | 4.3 / 5.6 / 5.6 | p95 | 11 | 3.7 | 24.8 | 0.5 | 0/19 | 6 / 6 | 154 MB | 3.59-3.62 | PASS |
| plain-10 | 10 | newline@middle | 19 | 4.4 / 5.2 / 5.5 | p95 | 11 | 3.5 | 19.9 | 0.0 | 19/19 | 3 / 3 | 154 MB | 3.62-3.6 | PASS |
| plain-10 | 10 | split@middle | 19 | 4.8 / 5.6 / 5.8 | p95 | 11 | 3.7 | 24.0 | 0.5 | 0/19 | 6 / 6 | 154 MB | 3.6-3.88 | PASS |
| plain-10 | 10 | typing@50ms | 39 | 4.3 / 6.5 / 11.3 | p95 | 11 | - | - | 1.9 | - | - | 154 MB | 3.88-3.88 | PASS |
| plain-10 | 10 | typing@60ms | 39 | 4.1 / 4.7 / 9.6 | p95 | 11 | - | - | 0.0 | - | - | 154 MB | 3.88-3.81 | PASS |
| plain-10 | 10 | typing@80ms | 39 | 4.4 / 4.7 / 4.8 | p95 | 11 | - | - | 0.2 | - | - | 154 MB | 3.81-3.81 | PASS |
| plain-10 | 10 | typing@100ms | 39 | 4.2 / 4.6 / 4.8 | p95 | 11 | - | - | 0.4 | - | - | 154 MB | 3.81-3.58 | PASS |
| plain-10 | 10 | typing@150ms | 39 | 4.6 / 5.4 / 5.5 | p95 | 11 | - | - | 0.5 | - | - | 154 MB | 3.58-3.45 | PASS |
| plain-10 | 10 | preamble | 13 | 36.6 / 38.6 / 54.9 | p95 | 400 | 38.6 | 52.9 | 0.3 | 0/13 | 10 / 10 | 154 MB | 3.45-3.5 | PASS |
| plain-10 | 10 | reopen: host's share, cold (spawn -> page 1) | 12 | 57.3 / 58.3 / 61.8 | p95 | (100, app) | - | - | - | - | - | 154 MB | 3.38-3.99 | report-only |
| plain-10 | 10 | reopen: host already listening (not counted, decision 8) | 12 | 18.4 / 19.0 / 19.6 | p95 | (100, app) | - | - | - | - | - | 154 MB | 3.38-3.99 | report-only |
| full-10 | 11 | letter@middle | 19 | 7.8 / 8.6 / 8.6 | p95 | 11 | 4.5 | 33.3 | 0.5 | 19/19 | 3 / 3 | 214 MB | 3.99-3.45 | PASS |
| full-10 | 11 | letter@end | 19 | 8.1 / 8.6 / 8.7 | p95 | 11 | 3.9 | 29.4 | 0.2 | 19/19 | 3 / 3 | 214 MB | 3.45-3.58 | PASS |
| full-10 | 11 | sentence@middle | 19 | 8.2 / 9.1 / 13.5 | p95 | 11 | 5.3 | 43.8 | 0.5 | 0/19 | 7 / 7 | 214 MB | 3.58-4.01 | PASS |
| full-10 | 11 | newline@middle | 19 | 7.4 / 8.5 / 8.7 | p95 | 11 | 4.6 | 33.7 | 0.0 | 19/19 | 3 / 3 | 214 MB | 4.01-3.7 | PASS |
| full-10 | 11 | split@middle | 19 | 7.8 / 9.5 / 9.8 | p95 | 11 | 5.0 | 34.9 | 0.5 | 19/19 | 3 / 3 | 214 MB | 3.7-3.81 | PASS |
| full-10 | 11 | letter@start | 19 | 13.5 / 14.2 / 15.0 | p95 | 11 | 14.2 | 47.0 | 1.0 | 19/19 | 2 / 2 | 214 MB | 3.81-3.74 | MISS: p95 14.2 ms > 11 ms |
| full-10 | 11 | typing@50ms | 39 | 7.8 / 8.2 / 8.3 | p95 | 11 | - | - | 0.0 | - | - | 214 MB | 3.74-3.6 | PASS |
| full-10 | 11 | typing@60ms | 39 | 6.5 / 7.5 / 11.7 | p95 | 11 | - | - | 0.0 | - | - | 214 MB | 3.6-3.6 | PASS |
| full-10 | 11 | typing@80ms | 39 | 7.1 / 8.2 / 8.8 | p95 | 11 | - | - | 0.2 | - | - | 214 MB | 3.6-4.27 | PASS |
| full-10 | 11 | typing@100ms | 39 | 8.1 / 8.9 / 9.0 | p95 | 11 | - | - | 0.2 | - | - | 214 MB | 4.27-4.17 | PASS |
| full-10 | 11 | typing@150ms | 39 | 8.0 / 8.7 / 9.3 | p95 | 11 | - | - | 0.3 | - | - | 214 MB | 4.17-4.0 | PASS |
| full-10 | 11 | preamble | 13 | 167.2 / 177.3 / 177.4 | p95 | 400 | 177.3 | 213.5 | 0.9 | 0/13 | 11 / 11 | 214 MB | 4.0-3.84 | PASS |
| full-10 | 11 | reopen: host's share, cold (spawn -> page 1) | 12 | 87.1 / 90.6 / 93.0 | p95 | (100, app) | - | - | - | - | - | 214 MB | 3.61-3.64 | report-only |
| full-10 | 11 | reopen: host already listening (not counted, decision 8) | 12 | 49.5 / 53.0 / 53.4 | p95 | (100, app) | - | - | - | - | - | 214 MB | 3.61-3.64 | report-only |
| plain-100 | 100 | letter@end | 19 | 5.1 / 6.0 / 6.7 | p95 | 11 | 4.1 | 21.5 | 0.5 | 19/19 | 3 / 3 | 189 MB | 3.64-3.46 | PASS |
| plain-100 | 100 | sentence@middle | 19 | 3.6 / 3.9 / 4.0 | p95 | 11 | 3.9 | 104.5 | 0.2 | 0/19 | 50 / 50 | 189 MB | 3.46-3.42 | PASS |
| plain-100 | 100 | newline@middle | 19 | 3.7 / 4.5 / 4.9 | p95 | 11 | 4.5 | 17.2 | 0.0 | 19/19 | 2 / 2 | 189 MB | 3.42-3.14 | PASS |
| plain-100 | 100 | split@middle | 19 | 4.0 / 4.9 / 5.0 | p95 | 11 | 4.9 | 19.1 | 0.5 | 19/19 | 2 / 2 | 189 MB | 3.14-2.89 | PASS |
| plain-100 | 100 | letter@start | 19 | 3.7 / 5.0 / 5.2 | p95 | 11 | 5.0 | 23.2 | 0.5 | 19/19 | 2 / 2 | 189 MB | 2.89-2.9 | PASS |
| plain-100 | 100 | letter@middle | 19 | 4.2 / 5.6 / 5.9 | p95 | 11 | 5.6 | 21.9 | 0.5 | 19/19 | 2 / 2 | 189 MB | 2.9-2.99 | PASS |
| plain-100 | 100 | typing@50ms | 39 | 3.2 / 3.4 / 3.7 | p95 | 11 | - | - | 0.0 | - | - | 189 MB | 2.99-3.15 | PASS |
| plain-100 | 100 | typing@60ms | 39 | 3.1 / 3.6 / 3.8 | p95 | 11 | - | - | 0.0 | - | - | 189 MB | 3.15-3.15 | PASS |
| plain-100 | 100 | typing@80ms | 39 | 3.4 / 3.6 / 3.6 | p95 | 11 | - | - | 0.3 | - | - | 189 MB | 3.15-3.22 | PASS |
| plain-100 | 100 | typing@100ms | 39 | 3.7 / 4.3 / 9.2 | p95 | 11 | - | - | 0.5 | - | - | 189 MB | 3.22-3.28 | PASS |
| plain-100 | 100 | typing@150ms | 39 | 3.8 / 4.6 / 4.9 | p95 | 11 | - | - | 0.5 | - | - | 189 MB | 3.28-3.1 | PASS |
| plain-100 | 100 | preamble | 13 | 34.9 / 37.4 / 37.5 | p95 | 400 | 37.4 | 153.7 | 0.5 | 0/13 | 100 / 100 | 189 MB | 3.1-3.17 | PASS |
| plain-100 | 100 | reopen: host's share, cold (spawn -> page 1) | 12 | 53.7 / 55.5 / 55.9 | p95 | (100, app) | - | - | - | - | - | 189 MB | 2.84-2.92 | report-only |
| plain-100 | 100 | reopen: host already listening (not counted, decision 8) | 12 | 16.4 / 17.5 / 17.8 | p95 | (100, app) | - | - | - | - | - | 189 MB | 2.84-2.92 | report-only |
| full-100 | 101 | sentence@middle | 19 | 6.8 / 8.5 / 9.3 | p95 | 11 | 8.5 | 32.2 | 0.5 | 19/19 | 2 / 2 | 232 MB | 2.84-3.91 | PASS |
| full-100 | 101 | newline@middle | 19 | 6.2 / 6.9 / 7.1 | p95 | 11 | 6.9 | 27.4 | 0.0 | 19/19 | 2 / 2 | 232 MB | 3.91-3.91 | PASS |
| full-100 | 101 | split@middle | 19 | 6.3 / 7.7 / 7.9 | p95 | 11 | 7.7 | 33.0 | 0.4 | 19/19 | 2 / 2 | 232 MB | 3.91-3.54 | PASS |
| full-100 | 101 | letter@start | 19 | 7.3 / 8.3 / 8.9 | p95 | 11 | 8.3 | 32.7 | 0.5 | 19/19 | 2 / 2 | 232 MB | 3.54-3.82 | PASS |
| full-100 | 101 | letter@middle | 19 | 6.9 / 7.9 / 8.9 | p95 | 11 | 7.9 | 31.2 | 0.6 | 19/19 | 2 / 2 | 232 MB | 3.82-3.75 | PASS |
| full-100 | 101 | letter@end | 19 | 8.5 / 9.6 / 9.9 | p95 | 11 | 5.3 | 32.2 | 0.6 | 19/19 | 3 / 3 | 232 MB | 3.75-3.41 | PASS |
| full-100 | 101 | typing@50ms | 39 | 5.1 / 5.9 / 13.4 | p95 | 11 | - | - | 0.0 | - | - | 232 MB | 3.41-3.41 | PASS |
| full-100 | 101 | typing@60ms | 39 | 5.4 / 5.9 / 6.3 | p95 | 11 | - | - | 0.0 | - | - | 232 MB | 3.41-3.29 | PASS |
| full-100 | 101 | typing@80ms | 39 | 6.1 / 6.3 / 6.5 | p95 | 11 | - | - | 0.1 | - | - | 232 MB | 3.29-3.29 | PASS |
| full-100 | 101 | typing@100ms | 39 | 6.0 / 6.6 / 6.7 | p95 | 11 | - | - | 0.3 | - | - | 232 MB | 3.29-3.35 | PASS |
| full-100 | 101 | typing@150ms | 39 | 6.2 / 6.8 / 7.1 | p95 | 11 | - | - | 0.5 | - | - | 232 MB | 3.35-3.4 | PASS |
| full-100 | 101 | preamble | 13 | 169.4 / 193.2 / 200.4 | p95 | 400 | 193.2 | 601.4 | 14.0 | 0/13 | 101 / 101 | 232 MB | 3.4-3.46 | PASS |
| full-100 | 101 | reopen: host's share, cold (spawn -> page 1) | 12 | 87.2 / 92.7 / 95.4 | p95 | (100, app) | - | - | - | - | - | 232 MB | 3.42-3.71 | report-only |
| full-100 | 101 | reopen: host already listening (not counted, decision 8) | 12 | 50.3 / 54.3 / 55.2 | p95 | (100, app) | - | - | - | - | - | 232 MB | 3.42-3.71 | report-only |
| plain-300 | 301 | newline@middle | 19 | 3.9 / 4.8 / 10.5 | p95 | 11 | 4.8 | 26.5 | 0.0 | 19/19 | 4 / 4 | 278 MB | 3.49-3.29 | PASS |
| plain-300 | 301 | split@middle | 19 | 4.1 / 5.2 / 5.7 | p95 | 11 | 5.2 | 39.6 | 0.4 | 19/19 | 9 / 9 | 278 MB | 3.29-3.4 | PASS |
| plain-300 | 301 | letter@start | 19 | 4.2 / 5.3 / 5.7 | p95 | 11 | 5.3 | 22.8 | 0.5 | 19/19 | 2 / 2 | 278 MB | 3.4-3.29 | PASS |
| plain-300 | 301 | letter@middle | 19 | 4.0 / 5.2 / 5.3 | p95 | 11 | 5.2 | 22.1 | 0.4 | 19/19 | 2 / 2 | 278 MB | 3.29-3.26 | PASS |
| plain-300 | 301 | letter@end | 19 | 3.7 / 5.1 / 5.5 | p95 | 11 | 5.1 | 20.4 | 0.4 | 19/19 | 2 / 2 | 278 MB | 3.26-3.44 | PASS |
| plain-300 | 301 | sentence@middle | 19 | 4.1 / 4.9 / 5.7 | p95 | 11 | 4.9 | 122.0 | 0.5 | 19/19 | 65 / 65 | 278 MB | 3.44-3.4 | PASS |
| plain-300 | 301 | typing@50ms | 39 | 3.3 / 4.3 / 4.5 | p95 | 11 | - | - | 0.1 | - | - | 278 MB | 3.4-3.29 | PASS |
| plain-300 | 301 | typing@60ms | 39 | 3.2 / 3.5 / 3.8 | p95 | 11 | - | - | 0.0 | - | - | 278 MB | 3.29-3.29 | PASS |
| plain-300 | 301 | typing@80ms | 39 | 3.5 / 3.8 / 3.9 | p95 | 11 | - | - | 0.3 | - | - | 278 MB | 3.29-3.19 | PASS |
| plain-300 | 301 | typing@100ms | 39 | 3.7 / 3.8 / 3.9 | p95 | 11 | - | - | 0.3 | - | - | 278 MB | 3.19-3.17 | PASS |
| plain-300 | 301 | typing@150ms | 39 | 4.1 / 4.5 / 4.7 | p95 | 11 | - | - | 0.5 | - | - | 278 MB | 3.17-3.08 | PASS |
| plain-300 | 301 | preamble | 13 | 36.7 / 37.8 / 40.1 | p95 | 400 | 37.8 | 402.5 | 0.5 | 0/13 | 301 / 301 | 278 MB | 3.08-2.84 | PASS |
| plain-300 | 301 | reopen: host's share, cold (spawn -> page 1) | 12 | 53.6 / 55.3 / 58.6 | p95 | (100, app) | - | - | - | - | - | 278 MB | 2.84-3.09 | report-only |
| plain-300 | 301 | reopen: host already listening (not counted, decision 8) | 12 | 16.7 / 16.9 / 18.5 | p95 | (100, app) | - | - | - | - | - | 278 MB | 2.84-3.09 | report-only |
| full-300 | 299 | split@middle | 19 | 7.4 / 8.4 / 9.3 | p95 | 11 | 8.4 | 183.4 | 0.1 | 19/19 | 33 / 33 | 337 MB | 2.84-6.98 | PASS |
| full-300 | 299 | letter@start | 19 | 6.3 / 7.7 / 12.9 | p95 | 11 | 7.7 | 46.4 | 0.1 | 19/19 | 2 / 2 | 337 MB | 6.98-7.22 | PASS |
| full-300 | 299 | letter@middle | 19 | 7.6 / 8.4 / 9.1 | p95 | 11 | 8.4 | 34.4 | 0.1 | 19/19 | 2 / 2 | 337 MB | 7.22-7.28 | PASS |
| full-300 | 299 | letter@end | 19 | 8.3 / 8.8 / 9.1 | p95 | 11 | 4.8 | 34.2 | 0.1 | 19/19 | 3 / 3 | 337 MB | 7.28-6.85 | PASS |
| full-300 | 299 | sentence@middle | 19 | 7.5 / 8.6 / 8.8 | p95 | 11 | 8.6 | 195.4 | 0.1 | 19/19 | 33 / 33 | 337 MB | 6.85-6.93 | PASS |
| full-300 | 299 | newline@middle | 19 | 7.8 / 8.5 / 8.8 | p95 | 11 | 8.5 | 32.2 | 0.0 | 19/19 | 2 / 2 | 337 MB | 6.93-6.77 | PASS |
| full-300 | 299 | typing@50ms | 39 | 6.4 / 7.3 / 11.7 | p95 | 11 | - | - | 0.0 | - | - | 337 MB | 6.77-6.77 | PASS |
| full-300 | 299 | typing@60ms | 39 | 6.5 / 7.0 / 7.3 | p95 | 11 | - | - | 0.0 | - | - | 337 MB | 6.77-6.79 | PASS |
| full-300 | 299 | typing@80ms | 39 | 6.9 / 7.9 / 8.9 | p95 | 11 | - | - | 0.2 | - | - | 337 MB | 6.79-6.79 | PASS |
| full-300 | 299 | typing@100ms | 39 | 7.2 / 8.0 / 8.1 | p95 | 11 | - | - | 0.3 | - | - | 337 MB | 6.79-6.65 | PASS |
| full-300 | 299 | typing@150ms | 39 | 7.4 / 8.1 / 8.2 | p95 | 11 | - | - | 0.3 | - | - | 337 MB | 6.65-6.52 | PASS |
| full-300 | 299 | preamble | 13 | 175.4 / 178.9 / 183.3 | p95 | 400 | 178.9 | 1465.4 | 0.6 | 0/13 | 299 / 299 | 337 MB | 6.52-5.32 | PASS |
| full-300 | 299 | reopen: host's share, cold (spawn -> page 1) | 12 | 91.8 / 94.4 / 95.2 | p95 | (100, app) | - | - | - | - | - | 337 MB | 4.81-5.13 | report-only |
| full-300 | 299 | reopen: host already listening (not counted, decision 8) | 12 | 53.1 / 54.5 / 55.3 | p95 | (100, app) | - | - | - | - | - | 337 MB | 4.81-5.13 | report-only |
| plain-1000 | 1001 | letter@start | 19 | 4.7 / 6.2 / 6.4 | p95 | 11 | 6.2 | 33.5 | 0.5 | 19/19 | 2 / 2 | 541 MB | 4.83-4.69 | PASS |
| plain-1000 | 1001 | letter@middle | 19 | 5.6 / 6.4 / 6.8 | p95 | 11 | 4.6 | 28.8 | 0.5 | 19/19 | 3 / 3 | 541 MB | 4.69-4.87 | PASS |
| plain-1000 | 1001 | letter@end | 19 | 4.4 / 5.3 / 5.9 | p95 | 11 | 5.3 | 22.5 | 0.5 | 19/19 | 2 / 2 | 541 MB | 4.87-5.28 | PASS |
| plain-1000 | 1001 | sentence@middle | 19 | 6.2 / 7.6 / 8.0 | p95 | 11 | 5.9 | 2968.1 | 0.5 | 0/19 | 1503 / 1505 | 541 MB | 5.28-4.83 | PASS |
| plain-1000 | 1001 | newline@middle | 19 | 6.2 / 7.0 / 7.7 | p95 | 11 | 5.3 | 35.3 | 0.0 | 19/19 | 3 / 3 | 541 MB | 4.83-4.7 | PASS |
| plain-1000 | 1001 | split@middle | 19 | 7.0 / 12.8 / 13.4 | p95 | 11 | 11.1 | 3232.1 | 5.9 | 0/19 | 1503 / 1505 | 541 MB | 4.7-5.39 | MISS: p95 12.8 ms > 11 ms |
| plain-1000 | 1001 | typing@50ms | 39 | 4.8 / 5.4 / 6.1 | p95 | 11 | - | - | 0.0 | - | - | 541 MB | 5.39-5.39 | PASS |
| plain-1000 | 1001 | typing@60ms | 39 | 4.8 / 5.1 / 8.6 | p95 | 11 | - | - | 0.0 | - | - | 541 MB | 5.39-5.35 | PASS |
| plain-1000 | 1001 | typing@80ms | 39 | 5.0 / 5.5 / 5.6 | p95 | 11 | - | - | 0.2 | - | - | 541 MB | 5.35-5.35 | PASS |
| plain-1000 | 1001 | typing@100ms | 39 | 5.2 / 5.5 / 8.5 | p95 | 11 | - | - | 0.3 | - | - | 541 MB | 5.35-5.57 | PASS |
| plain-1000 | 1001 | typing@150ms | 39 | 5.5 / 6.3 / 6.8 | p95 | 11 | - | - | 0.5 | - | - | 541 MB | 5.57-5.36 | PASS |
| plain-1000 | 1001 | preamble | 13 | 43.9 / 51.3 / 55.2 | p95 | 400 | 51.3 | 1425.3 | 0.5 | 0/13 | 1001 / 1001 | 541 MB | 5.36-5.26 | PASS |
| plain-1000 | 1001 | reopen: host's share, cold (spawn -> page 1) | 12 | 56.4 / 58.6 / 63.6 | p95 | (100, app) | - | - | - | - | - | 541 MB | 4.88-5.31 | report-only |
| plain-1000 | 1001 | reopen: host already listening (not counted, decision 8) | 12 | 18.7 / 20.2 / 23.2 | p95 | (100, app) | - | - | - | - | - | 541 MB | 4.88-5.31 | report-only |
| full-1000 | 1002 | letter@middle | 19 | 8.4 / 9.5 / 9.7 | p95 | 11 | 9.5 | 217.7 | 0.5 | 19/19 | 33 / 33 | 655 MB | 4.88-4.92 | PASS |
| full-1000 | 1002 | letter@end | 19 | 8.2 / 9.8 / 10.8 | p95 | 11 | 6.0 | 45.5 | 0.3 | 19/19 | 3 / 3 | 655 MB | 4.92-4.55 | PASS |
| full-1000 | 1002 | sentence@middle | 19 | 8.4 / 9.4 / 9.6 | p95 | 11 | 9.4 | 199.9 | 0.4 | 19/19 | 33 / 33 | 655 MB | 4.55-5.92 | PASS |
| full-1000 | 1002 | newline@middle | 19 | 8.9 / 10.7 / 10.8 | p95 | 11 | 10.7 | 56.2 | 0.0 | 19/19 | 3 / 3 | 655 MB | 5.92-5.61 | PASS |
| full-1000 | 1002 | split@middle | 19 | 9.6 / 10.8 / 11.2 | p95 | 11 | 10.8 | 232.1 | 0.5 | 19/19 | 33 / 33 | 655 MB | 5.61-5.13 | PASS |
| full-1000 | 1002 | letter@start | 19 | 9.9 / 11.1 / 14.5 | p95 | 11 | 11.1 | 58.2 | 0.5 | 19/19 | 2 / 2 | 655 MB | 5.13-5.12 | MISS: p95 11.1 ms > 11 ms |
| full-1000 | 1002 | typing@50ms | 39 | 10.9 / 13.2 / 15.6 | p95 | 11 | - | - | 1.0 | - | - | 655 MB | 5.12-5.27 | MISS: p95 13.2 ms > 11 ms |
| full-1000 | 1002 | typing@60ms | 39 | 9.6 / 11.9 / 12.4 | p95 | 11 | - | - | 0.4 | - | - | 655 MB | 5.27-5.27 | MISS: p95 11.9 ms > 11 ms |
| full-1000 | 1002 | typing@80ms | 39 | 9.5 / 11.7 / 12.0 | p95 | 11 | - | - | 0.5 | - | - | 655 MB | 5.27-5.17 | MISS: p95 11.7 ms > 11 ms |
| full-1000 | 1002 | typing@100ms | 39 | 9.7 / 12.8 / 14.0 | p95 | 11 | - | - | 0.5 | - | - | 655 MB | 5.17-5.23 | MISS: p95 12.8 ms > 11 ms |
| full-1000 | 1002 | typing@150ms | 39 | 10.3 / 12.3 / 16.0 | p95 | 11 | - | - | 0.5 | - | - | 655 MB | 5.23-5.14 | MISS: p95 12.3 ms > 11 ms |
| full-1000 | 1002 | preamble | 13 | 200.5 / 214.3 / 246.9 | p95 | 400 | 214.3 | 5082.9 | 10.6 | 0/13 | 1002 / 1002 | 655 MB | 5.14-5.39 | PASS |
| full-1000 | 1002 | reopen: host's share, cold (spawn -> page 1) | 12 | 101.6 / 107.7 / 109.3 | p95 | (100, app) | - | - | - | - | - | 655 MB | 5.43-6.79 | report-only: p95 107.7 ms > 100 ms |
| full-1000 | 1002 | reopen: host already listening (not counted, decision 8) | 12 | 62.1 / 65.6 / 67.0 | p95 | (100, app) | - | - | - | - | - | 655 MB | 5.43-6.79 | report-only |
| beamer-5 | 5 | letter@end | 19 | 12.0 / 16.3 / 17.3 | p95 | 11 | 16.3 | 55.0 | 0.4 | 19/19 | 2 / 2 | 202 MB | 5.93-5.7 | MISS: p95 16.3 ms > 11 ms (no baseline row) |
| beamer-5 | 5 | sentence@middle | 19 | 11.8 / 12.4 / 12.7 | p95 | 11 | 12.4 | 38.0 | 0.3 | 19/19 | 2 / 2 | 202 MB | 5.7-5.36 | MISS: p95 12.4 ms > 11 ms (no baseline row) |
| beamer-5 | 5 | newline@middle | 19 | 11.7 / 12.9 / 13.7 | p95 | 11 | 12.9 | 42.1 | 0.2 | 19/19 | 2 / 2 | 202 MB | 5.36-5.33 | MISS: p95 12.9 ms > 11 ms (no baseline row) |
| beamer-5 | 5 | split@middle | 19 | 11.9 / 12.5 / 12.8 | p95 | 11 | 12.5 | 45.9 | 0.4 | 19/19 | 2 / 2 | 202 MB | 5.33-5.19 | MISS: p95 12.5 ms > 11 ms (no baseline row) |
| beamer-5 | 5 | letter@start | 19 | 12.2 / 13.0 / 13.7 | p95 | 11 | 13.0 | 38.8 | 0.5 | 19/19 | 2 / 2 | 202 MB | 5.19-5.18 | MISS: p95 13.0 ms > 11 ms (no baseline row) |
| beamer-5 | 5 | letter@middle | 19 | 11.7 / 13.0 / 13.2 | p95 | 11 | 13.0 | 37.1 | 0.5 | 19/19 | 2 / 2 | 202 MB | 5.18-5.16 | MISS: p95 13.0 ms > 11 ms (no baseline row) |
| beamer-5 | 5 | typing@50ms | 39 | 11.2 / 12.7 / 12.9 | p95 | 11 | - | - | 0.1 | - | - | 202 MB | 5.16-5.07 | MISS: p95 12.7 ms > 11 ms |
| beamer-5 | 5 | typing@60ms | 39 | 10.8 / 12.0 / 12.0 | p95 | 11 | - | - | 0.0 | - | - | 202 MB | 5.07-5.07 | MISS: p95 12.0 ms > 11 ms |
| beamer-5 | 5 | typing@80ms | 39 | 10.9 / 12.0 / 12.3 | p95 | 11 | - | - | 0.0 | - | - | 202 MB | 5.07-4.82 | MISS: p95 12.0 ms > 11 ms |
| beamer-5 | 5 | typing@100ms | 39 | 11.5 / 12.5 / 13.0 | p95 | 11 | - | - | 0.3 | - | - | 202 MB | 4.82-4.68 | MISS: p95 12.5 ms > 11 ms |
| beamer-5 | 5 | typing@150ms | 39 | 11.5 / 12.3 / 18.6 | p95 | 11 | - | - | 0.3 | - | - | 202 MB | 4.68-4.62 | MISS: p95 12.3 ms > 11 ms |
| beamer-5 | 5 | preamble | 13 | 245.0 / 311.4 / 333.6 | p95 | 400 | 311.4 | 356.5 | 4.4 | 0/13 | 5 / 5 | 202 MB | 4.62-4.57 | PASS (no baseline row) |
| beamer-5 | 5 | reopen: host's share, cold (spawn -> page 1) | 12 | 86.0 / 93.8 / 93.9 | p95 | (100, app) | - | - | - | - | - | 202 MB | 4.37-4.42 | report-only |
| beamer-5 | 5 | reopen: host already listening (not counted, decision 8) | 12 | 47.9 / 52.4 / 53.8 | p95 | (100, app) | - | - | - | - | - | 202 MB | 4.37-4.42 | report-only |
| beamer-30 | 58 | sentence@middle | 19 | 24.0 / 26.9 / 26.9 | p95 | 11 | 26.9 | 61.9 | 0.5 | 19/19 | 2 / 2 | 217 MB | 4.42-4.57 | MISS: p95 26.9 ms > 11 ms (no baseline row) |
| beamer-30 | 58 | newline@middle | 19 | 23.5 / 25.7 / 26.4 | p95 | 11 | 25.7 | 78.9 | 0.1 | 19/19 | 3 / 3 | 217 MB | 4.57-4.42 | MISS: p95 25.7 ms > 11 ms (no baseline row) |
| beamer-30 | 58 | split@middle | 19 | 23.8 / 26.0 / 27.9 | p95 | 11 | 26.0 | 86.9 | 0.4 | 19/19 | 3 / 3 | 217 MB | 4.42-4.3 | MISS: p95 26.0 ms > 11 ms (no baseline row) |
| beamer-30 | 58 | letter@start | 19 | 25.2 / 27.0 / 27.7 | p95 | 11 | 27.0 | 64.1 | 0.5 | 19/19 | 2 / 2 | 217 MB | 4.3-4.4 | MISS: p95 27.0 ms > 11 ms (no baseline row) |
| beamer-30 | 58 | letter@middle | 19 | 24.4 / 26.3 / 26.9 | p95 | 11 | 26.3 | 62.2 | 0.5 | 19/19 | 2 / 2 | 217 MB | 4.4-4.29 | MISS: p95 26.3 ms > 11 ms (no baseline row) |
| beamer-30 | 58 | letter@end | 19 | 24.1 / 27.5 / 27.6 | p95 | 11 | 27.5 | 65.1 | 0.5 | 19/19 | 2 / 2 | 217 MB | 4.29-4.47 | MISS: p95 27.5 ms > 11 ms (no baseline row) |
| beamer-30 | 58 | typing@50ms | 39 | 26.3 / 37.9 / 42.6 | p95 | 11 | - | - | 2.0 | - | - | 217 MB | 4.47-4.47 | MISS: p95 37.9 ms > 11 ms |
| beamer-30 | 58 | typing@60ms | 39 | 22.9 / 33.4 / 36.0 | p95 | 11 | - | - | 5.6 | - | - | 217 MB | 4.47-4.91 | MISS: p95 33.4 ms > 11 ms |
| beamer-30 | 58 | typing@80ms | 39 | 22.8 / 24.8 / 24.9 | p95 | 11 | - | - | 0.1 | - | - | 217 MB | 4.91-4.91 | MISS: p95 24.8 ms > 11 ms |
| beamer-30 | 58 | typing@100ms | 39 | 23.8 / 26.5 / 30.4 | p95 | 11 | - | - | 0.2 | - | - | 217 MB | 4.91-5.08 | MISS: p95 26.5 ms > 11 ms |
| beamer-30 | 58 | typing@150ms | 39 | 23.4 / 24.9 / 25.4 | p95 | 11 | - | - | 0.3 | - | - | 217 MB | 5.08-4.99 | MISS: p95 24.9 ms > 11 ms |
| beamer-30 | 58 | preamble | 13 | 266.5 / 304.1 / 309.4 | p95 | 400 | 304.1 | 1196.0 | 20.8 | 0/13 | 58 / 58 | 217 MB | 4.99-5.21 | PASS (no baseline row) |
| beamer-30 | 58 | reopen: host's share, cold (spawn -> page 1) | 12 | 104.0 / 112.5 / 122.3 | p95 | (100, app) | - | - | - | - | - | 217 MB | 5.03-5.24 | report-only: p95 112.5 ms > 100 ms |
| beamer-30 | 58 | reopen: host already listening (not counted, decision 8) | 12 | 65.2 / 69.7 / 81.1 | p95 | (100, app) | - | - | - | - | - | 217 MB | 5.03-5.24 | report-only |

ms: COMPILE written to the watched page's PAGE frame read on the socket (preamble, reopen: page 1); typing@: the keystroke sent to the watched page from its own compile or a later one. off-CPU: of the host's time to that page, what its engine thread spent descheduled (wall minus queue minus thread CPU): load or a CPU quota, not the engine. The first sample of each phase is a warm-up, left out; rows with n < 12 are gated on their maximum. No noise margin: a row passes only when it meets its target.
