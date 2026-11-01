**NON-REFERENCE run**: on battery at start (73 %); on battery at end (62 %)

| doc | pages | edit | n | ms p50 / p95 / max | gated on | target | first changed page p95 | DONE p95 ms | off-CPU p95 ms | converged | re-typeset pages p50 / max | host RSS peak | load1 | verdict |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| plain-10 | 10 | letter@start | 19 | 5.1 / 8.2 / 11.4 | p95 | 11 | 8.2 | 26.4 | 0.8 | 19/19 | 2 / 2 | 169 MB | 4.31-3.94 | PASS |
| plain-10 | 10 | letter@middle | 19 | 3.8 / 4.7 / 5.1 | p95 | 11 | 3.1 | 18.1 | 0.2 | 19/19 | 3 / 3 | 169 MB | 3.94-3.71 | PASS |
| plain-10 | 10 | letter@end | 19 | 2.8 / 3.9 / 3.9 | p95 | 11 | 3.9 | 8.1 | 0.1 | 0/19 | 1 / 1 | 169 MB | 3.71-3.57 | PASS |
| plain-10 | 10 | sentence@middle | 19 | 3.6 / 3.8 / 3.9 | p95 | 11 | 2.1 | 21.6 | 0.1 | 0/19 | 6 / 6 | 169 MB | 3.57-3.44 | PASS |
| plain-10 | 10 | newline@middle | 19 | 3.8 / 4.4 / 4.6 | p95 | 11 | 2.7 | 18.5 | 0.1 | 19/19 | 3 / 3 | 169 MB | 3.44-3.22 | PASS |
| plain-10 | 10 | split@middle | 19 | 3.7 / 4.6 / 5.3 | p95 | 11 | 3.1 | 24.4 | 0.2 | 0/19 | 6 / 6 | 169 MB | 3.22-3.2 | PASS |
| plain-10 | 10 | typing@50ms | 39 | 3.1 / 3.8 / 7.5 | p95 | 11 | - | - | 0.1 | - | - | 169 MB | 3.2-3.2 | PASS |
| plain-10 | 10 | typing@60ms | 39 | 3.1 / 3.5 / 3.8 | p95 | 11 | - | - | 0.1 | - | - | 169 MB | 3.2-2.95 | PASS |
| plain-10 | 10 | typing@80ms | 39 | 3.7 / 5.6 / 6.7 | p95 | 11 | - | - | 0.3 | - | - | 169 MB | 2.95-3.43 | PASS |
| plain-10 | 10 | typing@100ms | 39 | 3.9 / 5.9 / 10.6 | p95 | 11 | - | - | 0.3 | - | - | 169 MB | 3.43-3.43 | PASS |
| plain-10 | 10 | typing@150ms | 39 | 3.9 / 4.5 / 4.6 | p95 | 11 | - | - | 0.2 | - | - | 169 MB | 3.43-3.46 | PASS |
| plain-10 | 10 | preamble | 13 | 33.4 / 35.4 / 42.1 | p95 | 400 | 35.4 | 55.5 | 1.1 | 0/13 | 10 / 10 | 169 MB | 3.46-3.26 | PASS |
| plain-10 | 10 | reopen: host's share, cold (spawn -> page 1) | 12 | 59.9 / 66.8 / 89.5 | p95 | (100, app) | - | - | - | - | - | 169 MB | 3.26-3.32 | report-only |
| plain-10 | 10 | reopen: host already listening (not counted, decision 8) | 12 | 18.6 / 21.5 / 33.0 | p95 | (100, app) | - | - | - | - | - | 169 MB | 3.26-3.32 | report-only |
| full-10 | 11 | letter@middle | 19 | 7.2 / 8.0 / 8.4 | p95 | 11 | 3.8 | 30.1 | 0.2 | 19/19 | 3 / 3 | 237 MB | 3.32-3.36 | PASS |
| full-10 | 11 | letter@end | 19 | 6.9 / 7.8 / 8.6 | p95 | 11 | 3.6 | 23.0 | 0.2 | 19/19 | 3 / 3 | 237 MB | 3.36-3.09 | PASS |
| full-10 | 11 | sentence@middle | 19 | 6.8 / 8.4 / 8.5 | p95 | 11 | 4.6 | 38.3 | 0.2 | 0/19 | 7 / 7 | 237 MB | 3.09-2.93 | PASS |
| full-10 | 11 | newline@middle | 19 | 6.9 / 7.5 / 8.7 | p95 | 11 | 3.5 | 29.1 | 0.2 | 19/19 | 3 / 3 | 237 MB | 2.93-2.78 | PASS |
| full-10 | 11 | split@middle | 19 | 7.0 / 8.5 / 8.7 | p95 | 11 | 4.1 | 33.2 | 0.2 | 19/19 | 3 / 3 | 237 MB | 2.78-2.63 | PASS |
| full-10 | 11 | letter@start | 19 | 8.0 / 8.7 / 8.9 | p95 | 11 | 8.7 | 35.1 | 0.6 | 19/19 | 2 / 2 | 237 MB | 2.63-2.82 | PASS |
| full-10 | 11 | typing@50ms | 39 | 6.5 / 7.5 / 8.4 | p95 | 11 | - | - | 0.2 | - | - | 237 MB | 2.82-2.68 | PASS |
| full-10 | 11 | typing@60ms | 39 | 6.5 / 8.2 / 9.7 | p95 | 11 | - | - | 0.2 | - | - | 237 MB | 2.68-2.68 | PASS |
| full-10 | 11 | typing@80ms | 39 | 6.6 / 7.3 / 7.4 | p95 | 11 | - | - | 0.2 | - | - | 237 MB | 2.68-2.78 | PASS |
| full-10 | 11 | typing@100ms | 39 | 6.7 / 7.5 / 8.9 | p95 | 11 | - | - | 0.2 | - | - | 237 MB | 2.78-3.04 | PASS |
| full-10 | 11 | typing@150ms | 39 | 7.0 / 9.4 / 11.0 | p95 | 11 | - | - | 0.2 | - | - | 237 MB | 3.04-4.64 | PASS |
| full-10 | 11 | preamble | 13 | 168.7 / 185.1 / 186.0 | p95 | 400 | 185.1 | 226.0 | 3.7 | 0/13 | 11 / 11 | 237 MB | 4.64-4.35 | PASS |
| full-10 | 11 | reopen: host's share, cold (spawn -> page 1) | 12 | 92.6 / 99.4 / 107.5 | p95 | (100, app) | - | - | - | - | - | 237 MB | 3.9-4.24 | report-only |
| full-10 | 11 | reopen: host already listening (not counted, decision 8) | 12 | 52.9 / 56.1 / 57.6 | p95 | (100, app) | - | - | - | - | - | 237 MB | 3.9-4.24 | report-only |
| plain-100 | 100 | letter@end | 19 | 4.0 / 9.4 / 12.6 | p95 | 11 | 8.9 | 22.5 | 0.1 | 19/19 | 3 / 3 | 196 MB | 3.9-3.46 | PASS |
| plain-100 | 100 | sentence@middle | 19 | 3.2 / 4.0 / 4.0 | p95 | 11 | 4.0 | 90.5 | 0.1 | 0/19 | 50 / 50 | 196 MB | 3.46-3.34 | PASS |
| plain-100 | 100 | newline@middle | 19 | 3.0 / 3.6 / 4.6 | p95 | 11 | 3.6 | 14.5 | 0.1 | 19/19 | 2 / 2 | 196 MB | 3.34-3.07 | PASS |
| plain-100 | 100 | split@middle | 19 | 3.0 / 3.4 / 5.5 | p95 | 11 | 3.4 | 19.4 | 0.2 | 19/19 | 2 / 2 | 196 MB | 3.07-2.9 | PASS |
| plain-100 | 100 | letter@start | 19 | 3.0 / 3.8 / 4.4 | p95 | 11 | 3.8 | 18.9 | 0.2 | 19/19 | 2 / 2 | 196 MB | 2.9-2.83 | PASS |
| plain-100 | 100 | letter@middle | 19 | 3.0 / 3.5 / 3.8 | p95 | 11 | 3.5 | 17.0 | 0.1 | 19/19 | 2 / 2 | 196 MB | 2.83-2.84 | PASS |
| plain-100 | 100 | typing@50ms | 39 | 2.7 / 3.1 / 4.3 | p95 | 11 | - | - | 0.1 | - | - | 196 MB | 2.84-2.77 | PASS |
| plain-100 | 100 | typing@60ms | 39 | 2.7 / 5.3 / 6.3 | p95 | 11 | - | - | 2.7 | - | - | 196 MB | 2.77-2.77 | PASS |
| plain-100 | 100 | typing@80ms | 39 | 2.8 / 3.0 / 3.6 | p95 | 11 | - | - | 0.1 | - | - | 196 MB | 2.77-2.55 | PASS |
| plain-100 | 100 | typing@100ms | 39 | 2.8 / 3.2 / 4.2 | p95 | 11 | - | - | 0.2 | - | - | 196 MB | 2.55-2.59 | PASS |
| plain-100 | 100 | typing@150ms | 39 | 2.9 / 3.1 / 3.3 | p95 | 11 | - | - | 0.1 | - | - | 196 MB | 2.59-2.46 | PASS |
| plain-100 | 100 | preamble | 13 | 33.7 / 38.6 / 43.7 | p95 | 400 | 38.6 | 162.5 | 1.2 | 0/13 | 100 / 100 | 196 MB | 2.46-2.42 | PASS |
| plain-100 | 100 | reopen: host's share, cold (spawn -> page 1) | 12 | 58.9 / 67.8 / 68.8 | p95 | (100, app) | - | - | - | - | - | 196 MB | 2.05-2.23 | report-only |
| plain-100 | 100 | reopen: host already listening (not counted, decision 8) | 12 | 17.6 / 19.7 / 22.4 | p95 | (100, app) | - | - | - | - | - | 196 MB | 2.05-2.23 | report-only |
| full-100 | 101 | sentence@middle | 19 | 5.6 / 7.0 / 13.6 | p95 | 11 | 7.0 | 32.4 | 0.2 | 19/19 | 2 / 2 | 288 MB | 2.05-2.03 | PASS |
| full-100 | 101 | newline@middle | 19 | 5.6 / 6.1 / 6.7 | p95 | 11 | 6.1 | 27.3 | 0.2 | 19/19 | 2 / 2 | 288 MB | 2.03-2.67 | PASS |
| full-100 | 101 | split@middle | 19 | 5.4 / 5.6 / 5.9 | p95 | 11 | 5.6 | 31.2 | 0.2 | 19/19 | 2 / 2 | 288 MB | 2.67-2.85 | PASS |
| full-100 | 101 | letter@start | 19 | 5.6 / 6.0 / 6.3 | p95 | 11 | 6.0 | 27.6 | 0.2 | 19/19 | 2 / 2 | 288 MB | 2.85-2.9 | PASS |
| full-100 | 101 | letter@middle | 19 | 5.4 / 7.2 / 8.4 | p95 | 11 | 7.2 | 33.6 | 0.3 | 19/19 | 2 / 2 | 288 MB | 2.9-5.07 | PASS |
| full-100 | 101 | letter@end | 19 | 7.6 / 8.2 / 9.6 | p95 | 11 | 3.8 | 31.3 | 0.2 | 19/19 | 3 / 3 | 288 MB | 5.07-5.06 | PASS |
| full-100 | 101 | typing@50ms | 39 | 5.1 / 5.5 / 6.1 | p95 | 11 | - | - | 0.2 | - | - | 288 MB | 5.06-4.81 | PASS |
| full-100 | 101 | typing@60ms | 39 | 5.0 / 5.6 / 7.4 | p95 | 11 | - | - | 0.2 | - | - | 288 MB | 4.81-4.81 | PASS |
| full-100 | 101 | typing@80ms | 39 | 5.2 / 6.1 / 9.9 | p95 | 11 | - | - | 0.3 | - | - | 288 MB | 4.81-4.59 | PASS |
| full-100 | 101 | typing@100ms | 39 | 5.1 / 5.8 / 9.1 | p95 | 11 | - | - | 0.2 | - | - | 288 MB | 4.59-4.38 | PASS |
| full-100 | 101 | typing@150ms | 39 | 5.4 / 5.9 / 5.9 | p95 | 11 | - | - | 0.2 | - | - | 288 MB | 4.38-4.19 | PASS |
| full-100 | 101 | preamble | 13 | 167.9 / 176.1 / 180.6 | p95 | 400 | 176.1 | 595.9 | 5.0 | 0/13 | 101 / 101 | 288 MB | 4.19-4.18 | PASS |
| full-100 | 101 | reopen: host's share, cold (spawn -> page 1) | 12 | 95.8 / 105.3 / 107.9 | p95 | (100, app) | - | - | - | - | - | 288 MB | 3.77-3.93 | report-only: p95 105.3 ms > 100 ms |
| full-100 | 101 | reopen: host already listening (not counted, decision 8) | 12 | 54.9 / 61.2 / 64.3 | p95 | (100, app) | - | - | - | - | - | 288 MB | 3.77-3.93 | report-only |
| plain-300 | 301 | newline@middle | 19 | 3.1 / 3.8 / 4.8 | p95 | 11 | 3.8 | 37.7 | 0.3 | 19/19 | 4 / 4 | 292 MB | 3.81-3.5 | PASS |
| plain-300 | 301 | split@middle | 19 | 2.9 / 3.2 / 4.5 | p95 | 11 | 3.2 | 42.7 | 0.1 | 19/19 | 9 / 9 | 292 MB | 3.5-3.26 | PASS |
| plain-300 | 301 | letter@start | 19 | 2.9 / 4.1 / 4.3 | p95 | 11 | 4.1 | 20.2 | 0.1 | 19/19 | 2 / 2 | 292 MB | 3.26-3.24 | PASS |
| plain-300 | 301 | letter@middle | 19 | 3.1 / 3.6 / 3.8 | p95 | 11 | 3.6 | 18.9 | 0.1 | 19/19 | 2 / 2 | 292 MB | 3.24-3.14 | PASS |
| plain-300 | 301 | letter@end | 19 | 3.1 / 3.3 / 3.3 | p95 | 11 | 3.3 | 16.3 | 0.1 | 19/19 | 2 / 2 | 292 MB | 3.14-2.89 | PASS |
| plain-300 | 301 | sentence@middle | 19 | 3.2 / 4.0 / 4.2 | p95 | 11 | 4.0 | 125.6 | 0.2 | 19/19 | 65 / 65 | 292 MB | 2.89-2.9 | PASS |
| plain-300 | 301 | typing@50ms | 39 | 2.8 / 3.5 / 10.9 | p95 | 11 | - | - | 0.4 | - | - | 292 MB | 2.9-3.07 | PASS |
| plain-300 | 301 | typing@60ms | 39 | 2.7 / 3.1 / 6.5 | p95 | 11 | - | - | 0.1 | - | - | 292 MB | 3.07-3.07 | PASS |
| plain-300 | 301 | typing@80ms | 39 | 2.7 / 3.0 / 3.1 | p95 | 11 | - | - | 0.1 | - | - | 292 MB | 3.07-3.06 | PASS |
| plain-300 | 301 | typing@100ms | 39 | 2.9 / 3.3 / 5.0 | p95 | 11 | - | - | 0.2 | - | - | 292 MB | 3.06-2.98 | PASS |
| plain-300 | 301 | typing@150ms | 39 | 3.0 / 3.3 / 3.7 | p95 | 11 | - | - | 0.1 | - | - | 292 MB | 2.98-2.9 | PASS |
| plain-300 | 301 | preamble | 13 | 36.4 / 41.8 / 50.6 | p95 | 400 | 41.8 | 509.3 | 1.4 | 0/13 | 301 / 301 | 292 MB | 2.9-4.11 | PASS |
| plain-300 | 301 | reopen: host's share, cold (spawn -> page 1) | 12 | 59.6 / 62.7 / 63.2 | p95 | (100, app) | - | - | - | - | - | 292 MB | 3.55-4.11 | report-only |
| plain-300 | 301 | reopen: host already listening (not counted, decision 8) | 12 | 18.2 / 19.4 / 21.7 | p95 | (100, app) | - | - | - | - | - | 292 MB | 3.55-4.11 | report-only |
| full-300 | 299 | split@middle | 19 | 6.9 / 7.6 / 8.4 | p95 | 11 | 7.6 | 185.7 | 0.2 | 19/19 | 33 / 33 | 448 MB | 3.55-3.33 | PASS |
| full-300 | 299 | letter@start | 19 | 5.5 / 6.0 / 6.2 | p95 | 11 | 6.0 | 32.9 | 0.2 | 19/19 | 2 / 2 | 448 MB | 3.33-3.06 | PASS |
| full-300 | 299 | letter@middle | 19 | 6.7 / 7.8 / 8.3 | p95 | 11 | 7.8 | 36.6 | 0.2 | 19/19 | 2 / 2 | 448 MB | 3.06-2.98 | PASS |
| full-300 | 299 | letter@end | 19 | 7.5 / 8.7 / 8.8 | p95 | 11 | 4.4 | 31.7 | 0.2 | 19/19 | 3 / 3 | 448 MB | 2.98-2.6 | PASS |
| full-300 | 299 | sentence@middle | 19 | 6.7 / 7.4 / 8.9 | p95 | 11 | 7.4 | 186.3 | 0.2 | 19/19 | 33 / 33 | 448 MB | 2.6-2.43 | PASS |
| full-300 | 299 | newline@middle | 19 | 7.4 / 8.8 / 10.0 | p95 | 11 | 8.8 | 38.3 | 0.4 | 19/19 | 2 / 2 | 448 MB | 2.43-2.24 | PASS |
| full-300 | 299 | typing@50ms | 39 | 6.2 / 6.8 / 7.2 | p95 | 11 | - | - | 0.2 | - | - | 448 MB | 2.24-2.24 | PASS |
| full-300 | 299 | typing@60ms | 39 | 6.5 / 7.2 / 7.4 | p95 | 11 | - | - | 0.2 | - | - | 448 MB | 2.24-2.22 | PASS |
| full-300 | 299 | typing@80ms | 39 | 6.6 / 7.1 / 7.6 | p95 | 11 | - | - | 0.2 | - | - | 448 MB | 2.22-2.2 | PASS |
| full-300 | 299 | typing@100ms | 39 | 6.4 / 6.9 / 7.3 | p95 | 11 | - | - | 0.2 | - | - | 448 MB | 2.2-2.2 | PASS |
| full-300 | 299 | typing@150ms | 39 | 6.6 / 7.3 / 8.2 | p95 | 11 | - | - | 0.2 | - | - | 448 MB | 2.2-2.26 | PASS |
| full-300 | 299 | preamble | 13 | 174.4 / 179.2 / 187.9 | p95 | 400 | 179.2 | 1542.2 | 2.7 | 0/13 | 299 / 299 | 448 MB | 2.26-2.36 | PASS |
| full-300 | 299 | reopen: host's share, cold (spawn -> page 1) | 12 | 96.8 / 99.5 / 106.5 | p95 | (100, app) | - | - | - | - | - | 448 MB | 2.31-2.49 | report-only |
| full-300 | 299 | reopen: host already listening (not counted, decision 8) | 12 | 56.0 / 57.6 / 63.4 | p95 | (100, app) | - | - | - | - | - | 448 MB | 2.31-2.49 | report-only |
| plain-1000 | 1001 | letter@start | 19 | 3.3 / 3.7 / 4.1 | p95 | 11 | 3.7 | 30.2 | 0.1 | 19/19 | 2 / 2 | 544 MB | 2.49-2.19 | PASS |
| plain-1000 | 1001 | letter@middle | 19 | 4.3 / 5.2 / 5.7 | p95 | 11 | 3.7 | 27.5 | 0.1 | 19/19 | 3 / 3 | 544 MB | 2.19-2.33 | PASS |
| plain-1000 | 1001 | letter@end | 19 | 3.2 / 4.0 / 4.6 | p95 | 11 | 4.0 | 18.2 | 0.2 | 19/19 | 2 / 2 | 544 MB | 2.33-2.23 | PASS |
| plain-1000 | 1001 | sentence@middle | 19 | 5.1 / 6.5 / 7.6 | p95 | 11 | 5.1 | 3014.8 | 0.2 | 0/19 | 1503 / 1505 | 544 MB | 2.23-2.65 | PASS |
| plain-1000 | 1001 | newline@middle | 19 | 4.7 / 4.9 / 5.3 | p95 | 11 | 3.4 | 26.0 | 0.1 | 19/19 | 3 / 3 | 544 MB | 2.65-2.8 | PASS |
| plain-1000 | 1001 | split@middle | 19 | 5.2 / 5.9 / 7.2 | p95 | 11 | 4.4 | 3033.8 | 0.2 | 0/19 | 1503 / 1505 | 544 MB | 2.8-2.29 | PASS |
| plain-1000 | 1001 | typing@50ms | 39 | 4.0 / 4.4 / 4.8 | p95 | 11 | - | - | 0.1 | - | - | 544 MB | 2.29-2.29 | PASS |
| plain-1000 | 1001 | typing@60ms | 39 | 4.0 / 4.4 / 4.6 | p95 | 11 | - | - | 0.2 | - | - | 544 MB | 2.29-2.43 | PASS |
| plain-1000 | 1001 | typing@80ms | 39 | 4.0 / 4.3 / 4.6 | p95 | 11 | - | - | 0.1 | - | - | 544 MB | 2.43-2.43 | PASS |
| plain-1000 | 1001 | typing@100ms | 39 | 4.4 / 5.8 / 7.2 | p95 | 11 | - | - | 0.3 | - | - | 544 MB | 2.43-2.39 | PASS |
| plain-1000 | 1001 | typing@150ms | 39 | 4.2 / 4.8 / 5.1 | p95 | 11 | - | - | 0.1 | - | - | 544 MB | 2.39-2.28 | PASS |
| plain-1000 | 1001 | preamble | 13 | 41.9 / 44.5 / 48.4 | p95 | 400 | 44.5 | 1313.3 | 1.2 | 0/13 | 1001 / 1001 | 544 MB | 2.28-3.27 | PASS |
| plain-1000 | 1001 | reopen: host's share, cold (spawn -> page 1) | 12 | 62.2 / 83.0 / 93.1 | p95 | (100, app) | - | - | - | - | - | 544 MB | 3.25-4.11 | report-only |
| plain-1000 | 1001 | reopen: host already listening (not counted, decision 8) | 12 | 20.0 / 31.6 / 35.9 | p95 | (100, app) | - | - | - | - | - | 544 MB | 3.25-4.11 | report-only |
| full-1000 | 1002 | letter@middle | 19 | 7.0 / 7.6 / 9.1 | p95 | 11 | 7.6 | 206.8 | 0.2 | 19/19 | 33 / 33 | 818 MB | 3.36-3.19 | PASS |
| full-1000 | 1002 | letter@end | 19 | 7.3 / 8.1 / 9.3 | p95 | 11 | 4.9 | 40.6 | 0.2 | 19/19 | 3 / 3 | 818 MB | 3.19-3.08 | PASS |
| full-1000 | 1002 | sentence@middle | 19 | 7.0 / 8.2 / 8.5 | p95 | 11 | 8.2 | 207.8 | 0.2 | 19/19 | 33 / 33 | 818 MB | 3.08-2.84 | PASS |
| full-1000 | 1002 | newline@middle | 19 | 7.4 / 8.7 / 8.9 | p95 | 11 | 8.7 | 48.5 | 0.2 | 19/19 | 3 / 3 | 818 MB | 2.84-2.69 | PASS |
| full-1000 | 1002 | split@middle | 19 | 7.1 / 7.5 / 8.2 | p95 | 11 | 7.5 | 209.5 | 0.2 | 19/19 | 33 / 33 | 818 MB | 2.69-2.67 | PASS |
| full-1000 | 1002 | letter@start | 19 | 7.4 / 10.3 / 11.2 | p95 | 11 | 10.3 | 58.2 | 2.0 | 19/19 | 2 / 2 | 818 MB | 2.67-2.49 | PASS |
| full-1000 | 1002 | typing@50ms | 39 | 7.5 / 14.1 / 23.7 | p95 | 11 | - | - | 2.7 | - | - | 818 MB | 2.49-2.49 | MISS: p95 14.1 ms > 11 ms |
| full-1000 | 1002 | typing@60ms | 39 | 7.6 / 8.6 / 8.9 | p95 | 11 | - | - | 0.4 | - | - | 818 MB | 2.49-2.49 | PASS |
| full-1000 | 1002 | typing@80ms | 39 | 7.3 / 8.2 / 8.9 | p95 | 11 | - | - | 0.4 | - | - | 818 MB | 2.49-2.45 | PASS |
| full-1000 | 1002 | typing@100ms | 39 | 7.4 / 8.8 / 9.3 | p95 | 11 | - | - | 0.4 | - | - | 818 MB | 2.45-2.33 | PASS |
| full-1000 | 1002 | typing@150ms | 39 | 7.5 / 8.2 / 8.9 | p95 | 11 | - | - | 0.4 | - | - | 818 MB | 2.33-2.31 | PASS |
| full-1000 | 1002 | preamble | 13 | 191.1 / 198.9 / 302.9 | p95 | 400 | 198.9 | 6516.2 | 7.1 | 0/13 | 1002 / 1002 | 818 MB | 2.31-2.81 | PASS |
| full-1000 | 1002 | reopen: host's share, cold (spawn -> page 1) | 12 | 108.5 / 251.8 / 1005.4 | p95 | (100, app) | - | - | - | - | - | 818 MB | 2.57-7.33 | report-only: p95 251.8 ms > 100 ms |
| full-1000 | 1002 | reopen: host already listening (not counted, decision 8) | 12 | 67.8 / 160.4 / 648.0 | p95 | (100, app) | - | - | - | - | - | 818 MB | 2.57-7.33 | report-only: p95 160.4 ms > 100 ms |
| beamer-5 | 5 | letter@end | 19 | 4.6 / 5.7 / 6.2 | p95 | 11 | 5.7 | 35.1 | 0.6 | 19/19 | 2 / 2 | 249 MB | 5.62-6.77 | PASS |
| beamer-5 | 5 | sentence@middle | 19 | 4.1 / 8.6 / 8.8 | p95 | 11 | 8.6 | 35.2 | 2.7 | 19/19 | 2 / 2 | 249 MB | 6.77-6.87 | PASS |
| beamer-5 | 5 | newline@middle | 19 | 4.2 / 5.3 / 5.4 | p95 | 11 | 5.3 | 30.4 | 0.2 | 19/19 | 2 / 3 | 249 MB | 6.87-6.6 | PASS |
| beamer-5 | 5 | split@middle | 19 | 4.0 / 6.4 / 20.9 | p95 | 11 | 6.4 | 53.6 | 0.3 | 19/19 | 3 / 3 | 249 MB | 6.6-6.23 | PASS |
| beamer-5 | 5 | letter@start | 19 | 4.3 / 6.0 / 6.7 | p95 | 11 | 6.0 | 32.4 | 1.2 | 19/19 | 2 / 2 | 249 MB | 6.23-6.06 | PASS |
| beamer-5 | 5 | letter@middle | 19 | 4.0 / 4.6 / 5.6 | p95 | 11 | 4.6 | 20.2 | 0.1 | 19/19 | 2 / 2 | 249 MB | 6.06-5.43 | PASS |
| beamer-5 | 5 | typing@50ms | 39 | 3.6 / 4.1 / 4.4 | p95 | 11 | - | - | 0.1 | - | - | 249 MB | 5.43-5.43 | PASS |
| beamer-5 | 5 | typing@60ms | 39 | 3.7 / 4.0 / 4.2 | p95 | 11 | - | - | 0.1 | - | - | 249 MB | 5.43-5.08 | PASS |
| beamer-5 | 5 | typing@80ms | 39 | 3.9 / 4.3 / 4.4 | p95 | 11 | - | - | 0.1 | - | - | 249 MB | 5.08-5.08 | PASS |
| beamer-5 | 5 | typing@100ms | 39 | 4.0 / 4.4 / 9.2 | p95 | 11 | - | - | 0.1 | - | - | 249 MB | 5.08-4.91 | PASS |
| beamer-5 | 5 | typing@150ms | 39 | 4.1 / 5.3 / 8.1 | p95 | 11 | - | - | 0.1 | - | - | 249 MB | 4.91-4.76 | PASS |
| beamer-5 | 5 | preamble | 13 | 239.3 / 254.1 / 262.2 | p95 | 400 | 254.1 | 300.6 | 5.5 | 0/13 | 5 / 5 | 249 MB | 4.76-5.07 | PASS |
| beamer-5 | 5 | reopen: host's share, cold (spawn -> page 1) | 12 | 99.1 / 102.5 / 104.1 | p95 | (100, app) | - | - | - | - | - | 249 MB | 4.66-5.07 | report-only: p95 102.5 ms > 100 ms |
| beamer-5 | 5 | reopen: host already listening (not counted, decision 8) | 12 | 55.3 / 58.1 / 58.4 | p95 | (100, app) | - | - | - | - | - | 249 MB | 4.66-5.07 | report-only |
| beamer-30 | 58 | sentence@middle | 19 | 7.8 / 8.7 / 9.4 | p95 | 11 | 8.7 | 43.5 | 0.1 | 19/19 | 2 / 2 | 312 MB | 4.66-4.25 | PASS |
| beamer-30 | 58 | newline@middle | 19 | 7.8 / 8.8 / 8.8 | p95 | 11 | 8.8 | 51.5 | 0.1 | 19/19 | 3 / 3 | 312 MB | 4.25-4.07 | PASS |
| beamer-30 | 58 | split@middle | 19 | 8.0 / 9.6 / 12.6 | p95 | 11 | 9.6 | 65.8 | 0.1 | 19/19 | 3 / 3 | 312 MB | 4.07-3.99 | PASS |
| beamer-30 | 58 | letter@start | 19 | 8.4 / 8.8 / 10.2 | p95 | 11 | 8.8 | 43.7 | 0.1 | 19/19 | 2 / 2 | 312 MB | 3.99-3.92 | PASS |
| beamer-30 | 58 | letter@middle | 19 | 7.8 / 9.1 / 10.7 | p95 | 11 | 9.1 | 32.4 | 0.1 | 19/19 | 2 / 2 | 312 MB | 3.92-4.33 | PASS |
| beamer-30 | 58 | letter@end | 19 | 7.8 / 8.4 / 8.8 | p95 | 11 | 8.4 | 41.9 | 0.1 | 19/19 | 2 / 2 | 312 MB | 4.33-3.96 | PASS |
| beamer-30 | 58 | typing@50ms | 39 | 7.4 / 7.9 / 8.1 | p95 | 11 | - | - | 0.1 | - | - | 312 MB | 3.96-3.96 | PASS |
| beamer-30 | 58 | typing@60ms | 39 | 7.5 / 8.0 / 8.2 | p95 | 11 | - | - | 0.1 | - | - | 312 MB | 3.96-3.96 | PASS |
| beamer-30 | 58 | typing@80ms | 39 | 7.5 / 8.0 / 8.4 | p95 | 11 | - | - | 0.1 | - | - | 312 MB | 3.96-3.73 | PASS |
| beamer-30 | 58 | typing@100ms | 39 | 7.7 / 8.4 / 8.4 | p95 | 11 | - | - | 0.1 | - | - | 312 MB | 3.73-3.51 | PASS |
| beamer-30 | 58 | typing@150ms | 39 | 7.6 / 8.2 / 8.5 | p95 | 11 | - | - | 0.1 | - | - | 312 MB | 3.51-3.47 | PASS |
| beamer-30 | 58 | preamble | 13 | 259.0 / 271.9 / 347.7 | p95 | 400 | 271.9 | 1182.5 | 5.6 | 0/13 | 58 / 58 | 312 MB | 3.47-3.11 | PASS |
| beamer-30 | 58 | reopen: host's share, cold (spawn -> page 1) | 12 | 111.4 / 115.7 / 125.6 | p95 | (100, app) | - | - | - | - | - | 312 MB | 2.94-3.03 | report-only: p95 115.7 ms > 100 ms |
| beamer-30 | 58 | reopen: host already listening (not counted, decision 8) | 12 | 70.7 / 73.9 / 84.0 | p95 | (100, app) | - | - | - | - | - | 312 MB | 2.94-3.03 | report-only |

ms: COMPILE written to the watched page's PAGE frame read on the socket (preamble, reopen: page 1); typing@: the keystroke sent to the watched page from its own compile or a later one. off-CPU: of the host's time to that page, what its engine thread spent descheduled (wall minus queue minus thread CPU): load or a CPU quota, not the engine. The first sample of each phase is a warm-up, left out; rows with n < 12 are gated on their maximum. No noise margin: a row passes only when it meets its target.
