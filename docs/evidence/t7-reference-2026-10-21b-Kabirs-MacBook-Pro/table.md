Reference run.

| doc | pages | edit | n | ms p50 / p95 / max | gated on | target | first changed page p95 | DONE p95 ms | off-CPU p95 ms | converged | re-typeset pages p50 / max | host RSS peak | load1 | verdict |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| plain-10 | 10 | letter@start | 19 | 7.1 / 9.8 / 10.5 | p95 | 11 | 9.8 | 26.5 | 1.2 | 19/19 | 2 / 2 | 190 MB | 5.27-4.85 | PASS |
| plain-10 | 10 | letter@middle | 19 | 4.7 / 5.8 / 5.8 | p95 | 11 | 4.1 | 17.7 | 0.8 | 19/19 | 3 / 3 | 190 MB | 4.85-4.41 | PASS |
| plain-10 | 10 | letter@end | 19 | 3.1 / 4.1 / 4.3 | p95 | 11 | 4.1 | 9.4 | 0.8 | 0/19 | 1 / 1 | 190 MB | 4.41-4.14 | PASS |
| plain-10 | 10 | sentence@middle | 19 | 4.2 / 5.3 / 5.7 | p95 | 11 | 3.7 | 22.2 | 0.8 | 0/19 | 6 / 6 | 190 MB | 4.14-4.13 | PASS |
| plain-10 | 10 | newline@middle | 19 | 4.1 / 5.1 / 5.8 | p95 | 11 | 3.4 | 16.9 | 0.1 | 19/19 | 3 / 3 | 190 MB | 4.13-3.8 | PASS |
| plain-10 | 10 | split@middle | 19 | 4.3 / 5.4 / 5.5 | p95 | 11 | 3.7 | 21.8 | 0.8 | 0/19 | 6 / 6 | 190 MB | 3.8-3.37 | PASS |
| plain-10 | 10 | typing@50ms | 39 | 3.1 / 3.7 / 3.9 | p95 | 11 | - | - | 0.2 | - | - | 190 MB | 3.37-3.37 | PASS |
| plain-10 | 10 | typing@60ms | 39 | 3.2 / 3.7 / 3.8 | p95 | 11 | - | - | 0.2 | - | - | 190 MB | 3.37-3.5 | PASS |
| plain-10 | 10 | typing@80ms | 39 | 3.6 / 4.0 / 4.1 | p95 | 11 | - | - | 0.5 | - | - | 190 MB | 3.5-3.5 | PASS |
| plain-10 | 10 | typing@100ms | 39 | 3.8 / 4.1 / 4.2 | p95 | 11 | - | - | 0.7 | - | - | 190 MB | 3.5-3.46 | PASS |
| plain-10 | 10 | typing@150ms | 39 | 4.2 / 4.8 / 5.8 | p95 | 11 | - | - | 0.8 | - | - | 190 MB | 3.46-3.26 | PASS |
| plain-10 | 10 | preamble | 13 | 36.5 / 50.3 / 59.7 | p95 | 400 | 50.3 | 70.7 | 2.7 | 0/13 | 10 / 10 | 190 MB | 3.26-3.0 | PASS |
| plain-10 | 10 | reopen: host's share, cold (spawn -> page 1) | 12 | 55.7 / 57.3 / 68.2 | p95 | (100, app) | - | - | - | - | - | 190 MB | 2.84-3.0 | report-only |
| plain-10 | 10 | reopen: host already listening (not counted, decision 8) | 12 | 17.1 / 17.3 / 28.7 | p95 | (100, app) | - | - | - | - | - | 190 MB | 2.84-3.0 | report-only |
| full-10 | 11 | letter@middle | 19 | 8.6 / 9.4 / 9.5 | p95 | 11 | 5.7 | 29.5 | 0.8 | 19/19 | 3 / 3 | 236 MB | 2.84-2.85 | PASS |
| full-10 | 11 | letter@end | 19 | 8.7 / 9.2 / 9.4 | p95 | 11 | 5.5 | 23.7 | 0.8 | 19/19 | 3 / 3 | 236 MB | 2.85-2.72 | PASS |
| full-10 | 11 | sentence@middle | 19 | 8.3 / 9.3 / 9.3 | p95 | 11 | 5.6 | 37.1 | 0.8 | 0/19 | 7 / 7 | 236 MB | 2.72-2.58 | PASS |
| full-10 | 11 | newline@middle | 19 | 8.1 / 8.7 / 8.9 | p95 | 11 | 5.0 | 30.0 | 0.2 | 19/19 | 3 / 3 | 236 MB | 2.58-2.37 | PASS |
| full-10 | 11 | split@middle | 19 | 8.3 / 8.9 / 8.9 | p95 | 11 | 5.2 | 33.3 | 0.8 | 19/19 | 3 / 3 | 236 MB | 2.37-2.17 | PASS |
| full-10 | 11 | letter@start | 19 | 9.7 / 11.2 / 15.4 | p95 | 11 | 11.2 | 32.0 | 1.3 | 19/19 | 2 / 2 | 236 MB | 2.17-2.07 | MISS: p95 11.2 ms > 11 ms |
| full-10 | 11 | typing@50ms | 39 | 6.3 / 6.5 / 7.1 | p95 | 11 | - | - | 0.2 | - | - | 236 MB | 2.07-2.07 | PASS |
| full-10 | 11 | typing@60ms | 39 | 6.4 / 7.4 / 7.6 | p95 | 11 | - | - | 0.2 | - | - | 236 MB | 2.07-2.31 | PASS |
| full-10 | 11 | typing@80ms | 39 | 6.6 / 7.7 / 11.5 | p95 | 11 | - | - | 0.5 | - | - | 236 MB | 2.31-2.12 | PASS |
| full-10 | 11 | typing@100ms | 39 | 7.1 / 10.7 / 11.6 | p95 | 11 | - | - | 0.5 | - | - | 236 MB | 2.12-2.12 | PASS |
| full-10 | 11 | typing@150ms | 39 | 7.6 / 8.3 / 8.4 | p95 | 11 | - | - | 0.8 | - | - | 236 MB | 2.12-2.11 | PASS |
| full-10 | 11 | preamble | 13 | 162.9 / 174.0 / 175.3 | p95 | 400 | 174.0 | 209.4 | 3.1 | 0/13 | 11 / 11 | 236 MB | 2.11-2.33 | PASS |
| full-10 | 11 | reopen: host's share, cold (spawn -> page 1) | 12 | 89.1 / 94.9 / 95.5 | p95 | (100, app) | - | - | - | - | - | 236 MB | 2.33-2.66 | report-only |
| full-10 | 11 | reopen: host already listening (not counted, decision 8) | 12 | 50.3 / 55.2 / 55.5 | p95 | (100, app) | - | - | - | - | - | 236 MB | 2.33-2.66 | report-only |
| plain-100 | 100 | letter@end | 19 | 4.7 / 5.7 / 6.0 | p95 | 11 | 4.1 | 17.3 | 0.8 | 19/19 | 3 / 3 | 195 MB | 2.66-2.77 | PASS |
| plain-100 | 100 | sentence@middle | 19 | 3.7 / 4.5 / 4.9 | p95 | 11 | 4.5 | 84.7 | 0.7 | 0/19 | 50 / 50 | 195 MB | 2.77-2.34 | PASS |
| plain-100 | 100 | newline@middle | 19 | 3.8 / 4.2 / 4.5 | p95 | 11 | 4.2 | 15.0 | 0.1 | 19/19 | 2 / 2 | 195 MB | 2.34-2.16 | PASS |
| plain-100 | 100 | split@middle | 19 | 4.5 / 4.9 / 5.0 | p95 | 11 | 4.9 | 18.2 | 0.8 | 19/19 | 2 / 2 | 195 MB | 2.16-2.06 | PASS |
| plain-100 | 100 | letter@start | 19 | 3.9 / 4.7 / 5.0 | p95 | 11 | 4.7 | 18.6 | 0.8 | 19/19 | 2 / 2 | 195 MB | 2.06-1.83 | PASS |
| plain-100 | 100 | letter@middle | 19 | 4.0 / 4.7 / 5.0 | p95 | 11 | 4.7 | 18.4 | 0.7 | 19/19 | 2 / 2 | 195 MB | 1.83-1.76 | PASS |
| plain-100 | 100 | typing@50ms | 39 | 2.6 / 2.8 / 2.9 | p95 | 11 | - | - | 0.2 | - | - | 195 MB | 1.76-1.76 | PASS |
| plain-100 | 100 | typing@60ms | 39 | 2.6 / 2.9 / 4.2 | p95 | 11 | - | - | 0.2 | - | - | 195 MB | 1.76-1.7 | PASS |
| plain-100 | 100 | typing@80ms | 39 | 3.0 / 3.2 / 3.4 | p95 | 11 | - | - | 0.5 | - | - | 195 MB | 1.7-1.7 | PASS |
| plain-100 | 100 | typing@100ms | 39 | 3.3 / 3.6 / 3.7 | p95 | 11 | - | - | 0.7 | - | - | 195 MB | 1.7-1.56 | PASS |
| plain-100 | 100 | typing@150ms | 39 | 3.8 / 4.2 / 4.4 | p95 | 11 | - | - | 0.8 | - | - | 195 MB | 1.56-1.44 | PASS |
| plain-100 | 100 | preamble | 13 | 36.8 / 37.8 / 38.7 | p95 | 400 | 37.8 | 154.1 | 1.6 | 0/13 | 100 / 100 | 195 MB | 1.44-1.52 | PASS |
| plain-100 | 100 | reopen: host's share, cold (spawn -> page 1) | 12 | 55.3 / 56.2 / 57.9 | p95 | (100, app) | - | - | - | - | - | 195 MB | 1.4-1.61 | report-only |
| plain-100 | 100 | reopen: host already listening (not counted, decision 8) | 12 | 16.7 / 17.8 / 18.6 | p95 | (100, app) | - | - | - | - | - | 195 MB | 1.4-1.61 | report-only |
| full-100 | 101 | sentence@middle | 19 | 6.4 / 7.4 / 7.7 | p95 | 11 | 7.4 | 28.3 | 0.8 | 19/19 | 2 / 2 | 285 MB | 1.61-1.64 | PASS |
| full-100 | 101 | newline@middle | 19 | 6.7 / 7.4 / 7.5 | p95 | 11 | 7.4 | 25.9 | 0.2 | 19/19 | 2 / 2 | 285 MB | 1.64-1.47 | PASS |
| full-100 | 101 | split@middle | 19 | 7.0 / 7.6 / 7.9 | p95 | 11 | 7.6 | 33.0 | 0.8 | 19/19 | 2 / 2 | 285 MB | 1.47-1.43 | PASS |
| full-100 | 101 | letter@start | 19 | 6.8 / 7.6 / 7.7 | p95 | 11 | 7.6 | 28.6 | 0.8 | 19/19 | 2 / 2 | 285 MB | 1.43-1.32 | PASS |
| full-100 | 101 | letter@middle | 19 | 6.2 / 7.4 / 7.9 | p95 | 11 | 7.4 | 27.7 | 0.8 | 19/19 | 2 / 2 | 285 MB | 1.32-1.27 | PASS |
| full-100 | 101 | letter@end | 19 | 8.6 / 9.8 / 9.9 | p95 | 11 | 5.8 | 30.4 | 0.8 | 19/19 | 3 / 3 | 285 MB | 1.27-1.25 | PASS |
| full-100 | 101 | typing@50ms | 39 | 4.9 / 5.4 / 6.6 | p95 | 11 | - | - | 0.2 | - | - | 285 MB | 1.25-1.25 | PASS |
| full-100 | 101 | typing@60ms | 39 | 5.0 / 5.8 / 6.0 | p95 | 11 | - | - | 0.2 | - | - | 285 MB | 1.25-1.39 | PASS |
| full-100 | 101 | typing@80ms | 39 | 5.2 / 5.6 / 5.8 | p95 | 11 | - | - | 0.5 | - | - | 285 MB | 1.39-1.39 | PASS |
| full-100 | 101 | typing@100ms | 39 | 5.5 / 5.9 / 6.3 | p95 | 11 | - | - | 0.5 | - | - | 285 MB | 1.39-1.27 | PASS |
| full-100 | 101 | typing@150ms | 39 | 6.3 / 6.7 / 8.2 | p95 | 11 | - | - | 0.8 | - | - | 285 MB | 1.27-1.25 | PASS |
| full-100 | 101 | preamble | 13 | 168.2 / 171.5 / 190.8 | p95 | 400 | 171.5 | 583.2 | 3.0 | 0/13 | 101 / 101 | 285 MB | 1.25-1.04 | PASS |
| full-100 | 101 | reopen: host's share, cold (spawn -> page 1) | 12 | 88.9 / 104.0 / 106.3 | p95 | (100, app) | - | - | - | - | - | 285 MB | 1.04-1.19 | report-only: p95 104.0 ms > 100 ms |
| full-100 | 101 | reopen: host already listening (not counted, decision 8) | 12 | 50.5 / 54.7 / 54.9 | p95 | (100, app) | - | - | - | - | - | 285 MB | 1.04-1.19 | report-only |
| plain-300 | 301 | newline@middle | 19 | 3.7 / 4.4 / 4.8 | p95 | 11 | 4.4 | 26.4 | 0.1 | 19/19 | 4 / 4 | 285 MB | 1.19-1.16 | PASS |
| plain-300 | 301 | split@middle | 19 | 4.8 / 5.3 / 5.5 | p95 | 11 | 5.3 | 38.7 | 0.8 | 19/19 | 9 / 9 | 285 MB | 1.16-1.15 | PASS |
| plain-300 | 301 | letter@start | 19 | 3.5 / 5.4 / 9.3 | p95 | 11 | 5.4 | 21.3 | 0.8 | 19/19 | 2 / 2 | 285 MB | 1.15-1.14 | PASS |
| plain-300 | 301 | letter@middle | 19 | 3.8 / 4.8 / 5.2 | p95 | 11 | 4.8 | 19.8 | 0.8 | 19/19 | 2 / 2 | 285 MB | 1.14-1.04 | PASS |
| plain-300 | 301 | letter@end | 19 | 4.5 / 5.0 / 5.2 | p95 | 11 | 5.0 | 18.0 | 0.8 | 19/19 | 2 / 2 | 285 MB | 1.04-0.96 | PASS |
| plain-300 | 301 | sentence@middle | 19 | 4.5 / 5.5 / 8.7 | p95 | 11 | 5.5 | 114.4 | 0.8 | 19/19 | 65 / 65 | 285 MB | 0.96-0.89 | PASS |
| plain-300 | 301 | typing@50ms | 39 | 2.7 / 2.9 / 6.3 | p95 | 11 | - | - | 0.2 | - | - | 285 MB | 0.89-0.89 | PASS |
| plain-300 | 301 | typing@60ms | 39 | 2.7 / 3.0 / 3.4 | p95 | 11 | - | - | 0.2 | - | - | 285 MB | 0.89-0.89 | PASS |
| plain-300 | 301 | typing@80ms | 39 | 3.1 / 3.3 / 6.1 | p95 | 11 | - | - | 0.4 | - | - | 285 MB | 0.89-1.06 | PASS |
| plain-300 | 301 | typing@100ms | 39 | 3.3 / 3.6 / 3.7 | p95 | 11 | - | - | 0.6 | - | - | 285 MB | 1.06-0.97 | PASS |
| plain-300 | 301 | typing@150ms | 39 | 3.4 / 4.1 / 4.1 | p95 | 11 | - | - | 0.7 | - | - | 285 MB | 0.97-0.98 | PASS |
| plain-300 | 301 | preamble | 13 | 38.4 / 53.8 / 61.8 | p95 | 400 | 53.8 | 430.6 | 14.8 | 0/13 | 301 / 301 | 285 MB | 0.98-1.6 | PASS |
| plain-300 | 301 | reopen: host's share, cold (spawn -> page 1) | 12 | 55.3 / 55.9 / 58.6 | p95 | (100, app) | - | - | - | - | - | 285 MB | 1.47-1.71 | report-only |
| plain-300 | 301 | reopen: host already listening (not counted, decision 8) | 12 | 17.1 / 17.7 / 18.4 | p95 | (100, app) | - | - | - | - | - | 285 MB | 1.47-1.71 | report-only |
| full-300 | 299 | split@middle | 19 | 8.2 / 9.5 / 9.6 | p95 | 11 | 9.5 | 175.9 | 0.8 | 19/19 | 33 / 33 | 439 MB | 1.71-1.7 | PASS |
| full-300 | 299 | letter@start | 19 | 7.2 / 8.3 / 8.6 | p95 | 11 | 8.3 | 33.1 | 0.8 | 19/19 | 2 / 2 | 439 MB | 1.7-1.64 | PASS |
| full-300 | 299 | letter@middle | 19 | 8.2 / 9.3 / 10.2 | p95 | 11 | 9.3 | 34.3 | 0.8 | 19/19 | 2 / 2 | 439 MB | 1.64-1.51 | PASS |
| full-300 | 299 | letter@end | 19 | 8.4 / 9.4 / 9.5 | p95 | 11 | 5.4 | 32.3 | 0.8 | 19/19 | 3 / 3 | 439 MB | 1.51-1.82 | PASS |
| full-300 | 299 | sentence@middle | 19 | 8.7 / 9.7 / 11.8 | p95 | 11 | 9.7 | 206.1 | 0.8 | 19/19 | 33 / 33 | 439 MB | 1.82-1.69 | PASS |
| full-300 | 299 | newline@middle | 19 | 7.7 / 8.7 / 8.9 | p95 | 11 | 8.7 | 29.5 | 0.2 | 19/19 | 2 / 2 | 439 MB | 1.69-1.64 | PASS |
| full-300 | 299 | typing@50ms | 39 | 6.4 / 7.3 / 7.3 | p95 | 11 | - | - | 0.2 | - | - | 439 MB | 1.64-1.64 | PASS |
| full-300 | 299 | typing@60ms | 39 | 6.4 / 6.6 / 7.4 | p95 | 11 | - | - | 0.2 | - | - | 439 MB | 1.64-1.58 | PASS |
| full-300 | 299 | typing@80ms | 39 | 6.5 / 6.8 / 7.2 | p95 | 11 | - | - | 0.2 | - | - | 439 MB | 1.58-1.54 | PASS |
| full-300 | 299 | typing@100ms | 39 | 6.9 / 7.3 / 7.5 | p95 | 11 | - | - | 0.5 | - | - | 439 MB | 1.54-1.54 | PASS |
| full-300 | 299 | typing@150ms | 39 | 7.5 / 8.2 / 8.4 | p95 | 11 | - | - | 0.7 | - | - | 439 MB | 1.54-1.45 | PASS |
| full-300 | 299 | preamble | 13 | 174.2 / 177.2 / 184.7 | p95 | 400 | 177.2 | 1427.1 | 3.0 | 0/13 | 299 / 299 | 439 MB | 1.45-1.63 | PASS |
| full-300 | 299 | reopen: host's share, cold (spawn -> page 1) | 12 | 92.6 / 99.9 / 100.3 | p95 | (100, app) | - | - | - | - | - | 439 MB | 1.49-1.88 | report-only |
| full-300 | 299 | reopen: host already listening (not counted, decision 8) | 12 | 54.0 / 57.9 / 59.4 | p95 | (100, app) | - | - | - | - | - | 439 MB | 1.49-1.88 | report-only |
| plain-1000 | 1001 | letter@start | 19 | 5.6 / 6.2 / 6.2 | p95 | 11 | 6.2 | 31.5 | 0.8 | 19/19 | 2 / 2 | 568 MB | 1.81-1.68 | PASS |
| plain-1000 | 1001 | letter@middle | 19 | 5.9 / 7.1 / 7.1 | p95 | 11 | 5.3 | 26.3 | 0.8 | 19/19 | 3 / 3 | 568 MB | 1.68-1.54 | PASS |
| plain-1000 | 1001 | letter@end | 19 | 4.7 / 5.5 / 5.7 | p95 | 11 | 5.5 | 19.5 | 0.8 | 19/19 | 2 / 2 | 568 MB | 1.54-1.62 | PASS |
| plain-1000 | 1001 | sentence@middle | 19 | 6.7 / 8.0 / 8.0 | p95 | 11 | 6.5 | 2900.7 | 0.8 | 0/19 | 1503 / 1505 | 568 MB | 1.62-1.61 | PASS |
| plain-1000 | 1001 | newline@middle | 19 | 5.8 / 7.1 / 7.2 | p95 | 11 | 5.7 | 27.6 | 0.1 | 19/19 | 3 / 3 | 568 MB | 1.61-1.44 | PASS |
| plain-1000 | 1001 | split@middle | 19 | 7.3 / 8.5 / 8.5 | p95 | 11 | 7.1 | 2845.3 | 0.8 | 0/19 | 1503 / 1505 | 568 MB | 1.44-1.84 | PASS |
| plain-1000 | 1001 | typing@50ms | 39 | 4.3 / 4.8 / 6.0 | p95 | 11 | - | - | 0.2 | - | - | 568 MB | 1.84-1.84 | PASS |
| plain-1000 | 1001 | typing@60ms | 39 | 4.3 / 4.5 / 5.0 | p95 | 11 | - | - | 0.2 | - | - | 568 MB | 1.84-1.78 | PASS |
| plain-1000 | 1001 | typing@80ms | 39 | 4.5 / 5.0 / 9.7 | p95 | 11 | - | - | 0.4 | - | - | 568 MB | 1.78-1.95 | PASS |
| plain-1000 | 1001 | typing@100ms | 39 | 4.5 / 5.0 / 5.2 | p95 | 11 | - | - | 0.4 | - | - | 568 MB | 1.95-1.95 | PASS |
| plain-1000 | 1001 | typing@150ms | 39 | 5.4 / 6.0 / 6.6 | p95 | 11 | - | - | 0.6 | - | - | 568 MB | 1.95-1.89 | PASS |
| plain-1000 | 1001 | preamble | 13 | 46.4 / 48.2 / 49.2 | p95 | 400 | 48.2 | 1242.5 | 1.6 | 0/13 | 1001 / 1001 | 568 MB | 1.89-1.86 | PASS |
| plain-1000 | 1001 | reopen: host's share, cold (spawn -> page 1) | 12 | 57.9 / 59.2 / 61.4 | p95 | (100, app) | - | - | - | - | - | 568 MB | 1.57-1.79 | report-only |
| plain-1000 | 1001 | reopen: host already listening (not counted, decision 8) | 12 | 19.2 / 20.3 / 20.4 | p95 | (100, app) | - | - | - | - | - | 568 MB | 1.57-1.79 | report-only |
| full-1000 | 1002 | letter@middle | 19 | 8.8 / 9.8 / 10.4 | p95 | 11 | 9.8 | 200.8 | 0.8 | 19/19 | 33 / 33 | 815 MB | 1.57-1.52 | PASS |
| full-1000 | 1002 | letter@end | 19 | 8.6 / 9.6 / 9.7 | p95 | 11 | 6.6 | 38.3 | 0.8 | 19/19 | 3 / 3 | 815 MB | 1.52-1.48 | PASS |
| full-1000 | 1002 | sentence@middle | 19 | 8.8 / 9.8 / 9.8 | p95 | 11 | 9.8 | 203.6 | 0.8 | 19/19 | 33 / 33 | 815 MB | 1.48-1.71 | PASS |
| full-1000 | 1002 | newline@middle | 19 | 8.8 / 9.5 / 9.7 | p95 | 11 | 9.5 | 49.8 | 0.1 | 19/19 | 3 / 3 | 815 MB | 1.71-1.65 | PASS |
| full-1000 | 1002 | split@middle | 19 | 9.3 / 10.2 / 10.4 | p95 | 11 | 10.2 | 214.9 | 0.8 | 19/19 | 33 / 33 | 815 MB | 1.65-1.85 | PASS |
| full-1000 | 1002 | letter@start | 19 | 10.1 / 10.9 / 11.1 | p95 | 11 | 10.9 | 50.9 | 0.8 | 19/19 | 2 / 2 | 815 MB | 1.85-1.8 | PASS |
| full-1000 | 1002 | typing@50ms | 39 | 9.0 / 10.3 / 10.5 | p95 | 11 | - | - | 1.0 | - | - | 815 MB | 1.8-1.8 | PASS |
| full-1000 | 1002 | typing@60ms | 39 | 9.2 / 13.4 / 14.2 | p95 | 11 | - | - | 1.1 | - | - | 815 MB | 1.8-2.54 | MISS: p95 13.4 ms > 11 ms |
| full-1000 | 1002 | typing@80ms | 39 | 9.1 / 9.9 / 10.0 | p95 | 11 | - | - | 0.6 | - | - | 815 MB | 2.54-2.41 | PASS |
| full-1000 | 1002 | typing@100ms | 39 | 9.4 / 12.2 / 12.3 | p95 | 11 | - | - | 0.6 | - | - | 815 MB | 2.41-2.41 | MISS: p95 12.2 ms > 11 ms |
| full-1000 | 1002 | typing@150ms | 39 | 9.6 / 10.7 / 10.9 | p95 | 11 | - | - | 0.6 | - | - | 815 MB | 2.41-2.34 | PASS |
| full-1000 | 1002 | preamble | 13 | 193.7 / 196.3 / 204.1 | p95 | 400 | 196.3 | 5160.9 | 3.1 | 0/13 | 1002 / 1002 | 815 MB | 2.34-1.88 | PASS |
| full-1000 | 1002 | reopen: host's share, cold (spawn -> page 1) | 12 | 101.2 / 108.7 / 115.4 | p95 | (100, app) | - | - | - | - | - | 815 MB | 1.53-1.85 | report-only: p95 108.7 ms > 100 ms |
| full-1000 | 1002 | reopen: host already listening (not counted, decision 8) | 12 | 62.9 / 67.0 / 71.3 | p95 | (100, app) | - | - | - | - | - | 815 MB | 1.53-1.85 | report-only |
| beamer-5 | 5 | letter@end | 19 | 4.9 / 6.2 / 6.3 | p95 | 11 | 6.2 | 28.6 | 0.7 | 19/19 | 2 / 2 | 252 MB | 1.53-1.49 | PASS |
| beamer-5 | 5 | sentence@middle | 19 | 5.4 / 6.0 / 6.0 | p95 | 11 | 6.0 | 20.5 | 0.7 | 19/19 | 2 / 2 | 252 MB | 1.49-1.45 | PASS |
| beamer-5 | 5 | newline@middle | 19 | 4.7 / 5.7 / 5.9 | p95 | 11 | 5.7 | 24.8 | 0.2 | 19/19 | 2 / 3 | 252 MB | 1.45-1.38 | PASS |
| beamer-5 | 5 | split@middle | 19 | 5.3 / 6.6 / 11.3 | p95 | 11 | 6.6 | 44.4 | 0.8 | 19/19 | 3 / 3 | 252 MB | 1.38-1.51 | PASS |
| beamer-5 | 5 | letter@start | 19 | 5.1 / 6.5 / 7.0 | p95 | 11 | 6.5 | 20.9 | 0.8 | 19/19 | 2 / 2 | 252 MB | 1.51-1.47 | PASS |
| beamer-5 | 5 | letter@middle | 19 | 5.5 / 6.2 / 6.4 | p95 | 11 | 6.2 | 20.3 | 0.7 | 19/19 | 2 / 2 | 252 MB | 1.47-1.32 | PASS |
| beamer-5 | 5 | typing@50ms | 39 | 3.3 / 3.5 / 3.7 | p95 | 11 | - | - | 0.1 | - | - | 252 MB | 1.32-1.32 | PASS |
| beamer-5 | 5 | typing@60ms | 39 | 3.3 / 4.0 / 4.2 | p95 | 11 | - | - | 0.1 | - | - | 252 MB | 1.32-1.22 | PASS |
| beamer-5 | 5 | typing@80ms | 39 | 3.8 / 4.1 / 9.6 | p95 | 11 | - | - | 0.4 | - | - | 252 MB | 1.22-1.22 | PASS |
| beamer-5 | 5 | typing@100ms | 39 | 4.1 / 4.4 / 4.5 | p95 | 11 | - | - | 0.6 | - | - | 252 MB | 1.22-1.52 | PASS |
| beamer-5 | 5 | typing@150ms | 39 | 4.6 / 5.3 / 5.6 | p95 | 11 | - | - | 0.7 | - | - | 252 MB | 1.52-1.56 | PASS |
| beamer-5 | 5 | preamble | 13 | 238.3 / 248.5 / 267.8 | p95 | 400 | 248.5 | 286.8 | 5.8 | 0/13 | 5 / 5 | 252 MB | 1.56-1.55 | PASS |
| beamer-5 | 5 | reopen: host's share, cold (spawn -> page 1) | 12 | 89.4 / 92.6 / 92.8 | p95 | (100, app) | - | - | - | - | - | 252 MB | 1.55-1.75 | report-only |
| beamer-5 | 5 | reopen: host already listening (not counted, decision 8) | 12 | 50.6 / 51.8 / 53.5 | p95 | (100, app) | - | - | - | - | - | 252 MB | 1.55-1.75 | report-only |
| beamer-30 | 58 | sentence@middle | 19 | 9.1 / 11.2 / 11.8 | p95 | 11 | 11.2 | 52.1 | 0.7 | 19/19 | 2 / 2 | 295 MB | 1.61-1.52 | MISS: p95 11.2 ms > 11 ms |
| beamer-30 | 58 | newline@middle | 19 | 8.3 / 9.6 / 12.2 | p95 | 11 | 9.6 | 45.0 | 0.1 | 19/19 | 3 / 3 | 295 MB | 1.52-1.56 | PASS |
| beamer-30 | 58 | split@middle | 19 | 9.4 / 9.8 / 10.0 | p95 | 11 | 9.8 | 61.7 | 0.7 | 19/19 | 3 / 3 | 295 MB | 1.56-1.51 | PASS |
| beamer-30 | 58 | letter@start | 19 | 9.4 / 11.6 / 12.2 | p95 | 11 | 11.6 | 44.7 | 0.8 | 19/19 | 2 / 2 | 295 MB | 1.51-1.59 | MISS: p95 11.6 ms > 11 ms |
| beamer-30 | 58 | letter@middle | 19 | 9.3 / 10.3 / 10.5 | p95 | 11 | 10.3 | 29.7 | 0.8 | 19/19 | 2 / 2 | 295 MB | 1.59-1.55 | PASS |
| beamer-30 | 58 | letter@end | 19 | 9.2 / 10.8 / 11.6 | p95 | 11 | 10.8 | 44.8 | 0.8 | 19/19 | 2 / 2 | 295 MB | 1.55-1.38 | PASS |
| beamer-30 | 58 | typing@50ms | 39 | 6.8 / 7.4 / 8.1 | p95 | 11 | - | - | 0.1 | - | - | 295 MB | 1.38-1.38 | PASS |
| beamer-30 | 58 | typing@60ms | 39 | 6.9 / 7.9 / 8.1 | p95 | 11 | - | - | 0.1 | - | - | 295 MB | 1.38-1.38 | PASS |
| beamer-30 | 58 | typing@80ms | 39 | 7.1 / 7.5 / 7.6 | p95 | 11 | - | - | 0.4 | - | - | 295 MB | 1.38-1.35 | PASS |
| beamer-30 | 58 | typing@100ms | 39 | 7.5 / 8.1 / 8.2 | p95 | 11 | - | - | 0.4 | - | - | 295 MB | 1.35-1.56 | PASS |
| beamer-30 | 58 | typing@150ms | 39 | 8.2 / 9.2 / 9.2 | p95 | 11 | - | - | 0.7 | - | - | 295 MB | 1.56-1.68 | PASS |
| beamer-30 | 58 | preamble | 13 | 268.4 / 339.5 / 353.9 | p95 | 400 | 339.5 | 1183.7 | 11.4 | 0/13 | 58 / 58 | 295 MB | 1.68-1.72 | PASS |
| beamer-30 | 58 | reopen: host's share, cold (spawn -> page 1) | 12 | 103.7 / 107.1 / 107.7 | p95 | (100, app) | - | - | - | - | - | 295 MB | 1.7-1.83 | report-only: p95 107.1 ms > 100 ms |
| beamer-30 | 58 | reopen: host already listening (not counted, decision 8) | 12 | 65.0 / 68.0 / 68.8 | p95 | (100, app) | - | - | - | - | - | 295 MB | 1.7-1.83 | report-only |

ms: COMPILE written to the watched page's PAGE frame read on the socket (preamble, reopen: page 1); typing@: the keystroke sent to the watched page from its own compile or a later one. off-CPU: of the host's time to that page, what its engine thread spent descheduled (wall minus queue minus thread CPU): load or a CPU quota, not the engine. The first sample of each phase is a warm-up, left out; rows with n < 12 are gated on their maximum. No noise margin: a row passes only when it meets its target.
