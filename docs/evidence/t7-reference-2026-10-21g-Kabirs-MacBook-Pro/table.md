**NON-REFERENCE run**: on battery at end (73 %)

| doc | pages | edit | n | ms p50 / p95 / max | gated on | target | first changed page p95 | DONE p95 ms | off-CPU p95 ms | converged | re-typeset pages p50 / max | host RSS peak | load1 | verdict |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| plain-10 | 10 | letter@start | 19 | 4.5 / 8.7 / 9.1 | p95 | 11 | 8.7 | 23.8 | 0.5 | 19/19 | 2 / 2 | 169 MB | 3.71-3.29 | PASS |
| plain-10 | 10 | letter@middle | 19 | 3.8 / 4.9 / 5.7 | p95 | 11 | 3.5 | 16.4 | 0.1 | 19/19 | 3 / 3 | 169 MB | 3.29-3.03 | PASS |
| plain-10 | 10 | letter@end | 19 | 2.7 / 3.2 / 3.9 | p95 | 11 | 3.2 | 7.8 | 0.1 | 0/19 | 1 / 1 | 169 MB | 3.03-3.1 | PASS |
| plain-10 | 10 | sentence@middle | 19 | 3.6 / 4.1 / 4.6 | p95 | 11 | 2.6 | 21.3 | 0.1 | 0/19 | 6 / 6 | 169 MB | 3.1-2.94 | PASS |
| plain-10 | 10 | newline@middle | 19 | 3.7 / 4.3 / 5.0 | p95 | 11 | 2.8 | 16.8 | 0.1 | 19/19 | 3 / 3 | 169 MB | 2.94-2.86 | PASS |
| plain-10 | 10 | split@middle | 19 | 3.8 / 4.6 / 6.2 | p95 | 11 | 3.1 | 22.7 | 0.1 | 0/19 | 6 / 6 | 169 MB | 2.86-2.71 | PASS |
| plain-10 | 10 | typing@50ms | 39 | 3.3 / 3.8 / 6.4 | p95 | 11 | - | - | 0.1 | - | - | 169 MB | 2.71-2.71 | PASS |
| plain-10 | 10 | typing@60ms | 39 | 3.5 / 4.0 / 9.7 | p95 | 11 | - | - | 0.1 | - | - | 169 MB | 2.71-2.57 | PASS |
| plain-10 | 10 | typing@80ms | 39 | 3.4 / 3.8 / 4.5 | p95 | 11 | - | - | 0.1 | - | - | 169 MB | 2.57-2.57 | PASS |
| plain-10 | 10 | typing@100ms | 39 | 3.7 / 3.9 / 4.0 | p95 | 11 | - | - | 0.1 | - | - | 169 MB | 2.57-6.21 | PASS |
| plain-10 | 10 | typing@150ms | 39 | 3.7 / 4.6 / 4.7 | p95 | 11 | - | - | 0.1 | - | - | 169 MB | 6.21-5.87 | PASS |
| plain-10 | 10 | preamble | 13 | 33.1 / 40.7 / 61.6 | p95 | 400 | 40.7 | 53.7 | 6.6 | 0/13 | 10 / 10 | 169 MB | 5.87-5.64 | PASS |
| plain-10 | 10 | reopen: host's share, cold (spawn -> page 1) | 12 | 57.0 / 59.6 / 59.8 | p95 | (100, app) | - | - | - | - | - | 169 MB | 4.93-5.27 | report-only |
| plain-10 | 10 | reopen: host already listening (not counted, decision 8) | 12 | 17.1 / 18.5 / 18.6 | p95 | (100, app) | - | - | - | - | - | 169 MB | 4.93-5.27 | report-only |
| full-10 | 11 | letter@middle | 19 | 6.8 / 7.4 / 8.1 | p95 | 11 | 3.8 | 29.7 | 0.2 | 19/19 | 3 / 3 | 240 MB | 4.93-4.24 | PASS |
| full-10 | 11 | letter@end | 19 | 6.9 / 8.6 / 8.9 | p95 | 11 | 4.8 | 21.7 | 0.2 | 19/19 | 3 / 3 | 240 MB | 4.24-3.9 | PASS |
| full-10 | 11 | sentence@middle | 19 | 7.4 / 8.5 / 8.6 | p95 | 11 | 4.7 | 37.2 | 0.2 | 0/19 | 7 / 7 | 240 MB | 3.9-3.67 | PASS |
| full-10 | 11 | newline@middle | 19 | 7.2 / 8.5 / 8.6 | p95 | 11 | 4.8 | 29.6 | 0.2 | 19/19 | 3 / 3 | 240 MB | 3.67-3.19 | PASS |
| full-10 | 11 | split@middle | 19 | 7.4 / 8.5 / 8.9 | p95 | 11 | 4.8 | 33.4 | 0.2 | 19/19 | 3 / 3 | 240 MB | 3.19-3.01 | PASS |
| full-10 | 11 | letter@start | 19 | 7.6 / 8.3 / 9.2 | p95 | 11 | 8.3 | 29.0 | 0.7 | 19/19 | 2 / 2 | 240 MB | 3.01-2.85 | PASS |
| full-10 | 11 | typing@50ms | 39 | 6.6 / 7.7 / 9.0 | p95 | 11 | - | - | 0.2 | - | - | 240 MB | 2.85-2.7 | PASS |
| full-10 | 11 | typing@60ms | 39 | 6.8 / 10.8 / 11.3 | p95 | 11 | - | - | 0.6 | - | - | 240 MB | 2.7-2.7 | PASS |
| full-10 | 11 | typing@80ms | 39 | 6.8 / 7.6 / 7.9 | p95 | 11 | - | - | 0.2 | - | - | 240 MB | 2.7-2.56 | PASS |
| full-10 | 11 | typing@100ms | 39 | 6.7 / 7.4 / 8.1 | p95 | 11 | - | - | 0.2 | - | - | 240 MB | 2.56-2.36 | PASS |
| full-10 | 11 | typing@150ms | 39 | 6.8 / 7.5 / 10.8 | p95 | 11 | - | - | 0.2 | - | - | 240 MB | 2.36-2.25 | PASS |
| full-10 | 11 | preamble | 13 | 159.0 / 162.2 / 174.2 | p95 | 400 | 162.2 | 200.2 | 2.4 | 0/13 | 11 / 11 | 240 MB | 2.25-2.15 | PASS |
| full-10 | 11 | reopen: host's share, cold (spawn -> page 1) | 12 | 90.3 / 93.2 / 103.2 | p95 | (100, app) | - | - | - | - | - | 240 MB | 2.04-2.22 | report-only |
| full-10 | 11 | reopen: host already listening (not counted, decision 8) | 12 | 50.8 / 54.8 / 56.3 | p95 | (100, app) | - | - | - | - | - | 240 MB | 2.04-2.22 | report-only |
| plain-100 | 100 | letter@end | 19 | 4.0 / 5.2 / 5.3 | p95 | 11 | 3.7 | 16.1 | 0.1 | 19/19 | 3 / 3 | 197 MB | 2.04-1.81 | PASS |
| plain-100 | 100 | sentence@middle | 19 | 2.9 / 3.7 / 3.8 | p95 | 11 | 3.7 | 83.5 | 0.1 | 0/19 | 50 / 50 | 197 MB | 1.81-1.74 | PASS |
| plain-100 | 100 | newline@middle | 19 | 3.0 / 3.7 / 3.7 | p95 | 11 | 3.7 | 13.9 | 0.1 | 19/19 | 2 / 2 | 197 MB | 1.74-1.68 | PASS |
| plain-100 | 100 | split@middle | 19 | 3.0 / 3.8 / 5.7 | p95 | 11 | 3.8 | 17.6 | 0.1 | 19/19 | 2 / 2 | 197 MB | 1.68-1.5 | PASS |
| plain-100 | 100 | letter@start | 19 | 2.9 / 4.3 / 4.4 | p95 | 11 | 4.3 | 20.4 | 0.4 | 19/19 | 2 / 2 | 197 MB | 1.5-1.46 | PASS |
| plain-100 | 100 | letter@middle | 19 | 2.9 / 4.0 / 4.5 | p95 | 11 | 4.0 | 18.0 | 0.1 | 19/19 | 2 / 2 | 197 MB | 1.46-1.34 | PASS |
| plain-100 | 100 | typing@50ms | 39 | 2.5 / 2.8 / 3.0 | p95 | 11 | - | - | 0.1 | - | - | 197 MB | 1.34-1.23 | PASS |
| plain-100 | 100 | typing@60ms | 39 | 2.7 / 2.9 / 4.1 | p95 | 11 | - | - | 0.1 | - | - | 197 MB | 1.23-1.23 | PASS |
| plain-100 | 100 | typing@80ms | 39 | 2.7 / 3.1 / 3.2 | p95 | 11 | - | - | 0.1 | - | - | 197 MB | 1.23-1.21 | PASS |
| plain-100 | 100 | typing@100ms | 39 | 2.8 / 3.1 / 4.7 | p95 | 11 | - | - | 0.1 | - | - | 197 MB | 1.21-1.12 | PASS |
| plain-100 | 100 | typing@150ms | 39 | 2.9 / 4.3 / 4.6 | p95 | 11 | - | - | 0.4 | - | - | 197 MB | 1.12-1.03 | PASS |
| plain-100 | 100 | preamble | 13 | 33.3 / 33.6 / 35.4 | p95 | 400 | 33.6 | 157.6 | 1.1 | 0/13 | 100 / 100 | 197 MB | 1.03-1.02 | PASS |
| plain-100 | 100 | reopen: host's share, cold (spawn -> page 1) | 12 | 55.1 / 56.8 / 57.8 | p95 | (100, app) | - | - | - | - | - | 197 MB | 1.02-1.25 | report-only |
| plain-100 | 100 | reopen: host already listening (not counted, decision 8) | 12 | 16.5 / 17.1 / 17.2 | p95 | (100, app) | - | - | - | - | - | 197 MB | 1.02-1.25 | report-only |
| full-100 | 101 | sentence@middle | 19 | 5.4 / 6.4 / 7.0 | p95 | 11 | 6.4 | 28.1 | 0.2 | 19/19 | 2 / 2 | 288 MB | 1.25-1.36 | PASS |
| full-100 | 101 | newline@middle | 19 | 5.6 / 6.9 / 7.2 | p95 | 11 | 6.9 | 25.4 | 0.2 | 19/19 | 2 / 2 | 288 MB | 1.36-1.33 | PASS |
| full-100 | 101 | split@middle | 19 | 5.5 / 7.2 / 7.4 | p95 | 11 | 7.2 | 39.6 | 0.3 | 19/19 | 2 / 2 | 288 MB | 1.33-2.34 | PASS |
| full-100 | 101 | letter@start | 19 | 5.4 / 7.2 / 7.2 | p95 | 11 | 7.2 | 27.6 | 0.3 | 19/19 | 2 / 2 | 288 MB | 2.34-2.2 | PASS |
| full-100 | 101 | letter@middle | 19 | 5.4 / 6.5 / 7.3 | p95 | 11 | 6.5 | 26.6 | 0.2 | 19/19 | 2 / 2 | 288 MB | 2.2-2.03 | PASS |
| full-100 | 101 | letter@end | 19 | 7.1 / 9.1 / 9.7 | p95 | 11 | 4.9 | 27.5 | 0.1 | 19/19 | 3 / 3 | 288 MB | 2.03-2.42 | PASS |
| full-100 | 101 | typing@50ms | 39 | 5.1 / 5.6 / 5.9 | p95 | 11 | - | - | 0.2 | - | - | 288 MB | 2.42-2.31 | PASS |
| full-100 | 101 | typing@60ms | 39 | 5.0 / 5.4 / 5.5 | p95 | 11 | - | - | 0.2 | - | - | 288 MB | 2.31-2.31 | PASS |
| full-100 | 101 | typing@80ms | 39 | 5.0 / 5.7 / 7.4 | p95 | 11 | - | - | 0.2 | - | - | 288 MB | 2.31-2.36 | PASS |
| full-100 | 101 | typing@100ms | 39 | 5.1 / 5.8 / 6.5 | p95 | 11 | - | - | 0.2 | - | - | 288 MB | 2.36-2.26 | PASS |
| full-100 | 101 | typing@150ms | 39 | 5.3 / 6.0 / 6.5 | p95 | 11 | - | - | 0.2 | - | - | 288 MB | 2.26-2.07 | PASS |
| full-100 | 101 | preamble | 13 | 163.4 / 179.2 / 215.3 | p95 | 400 | 179.2 | 619.9 | 6.2 | 0/13 | 101 / 101 | 288 MB | 2.07-1.98 | PASS |
| full-100 | 101 | reopen: host's share, cold (spawn -> page 1) | 12 | 92.2 / 105.5 / 106.9 | p95 | (100, app) | - | - | - | - | - | 288 MB | 1.62-1.82 | report-only: p95 105.5 ms > 100 ms |
| full-100 | 101 | reopen: host already listening (not counted, decision 8) | 12 | 52.6 / 55.7 / 56.9 | p95 | (100, app) | - | - | - | - | - | 288 MB | 1.62-1.82 | report-only |
| plain-300 | 301 | newline@middle | 19 | 3.0 / 3.4 / 4.6 | p95 | 11 | 3.4 | 27.0 | 0.1 | 19/19 | 4 / 4 | 292 MB | 1.65-1.6 | PASS |
| plain-300 | 301 | split@middle | 19 | 3.0 / 3.6 / 3.9 | p95 | 11 | 3.6 | 37.8 | 0.1 | 19/19 | 9 / 9 | 292 MB | 1.6-1.58 | PASS |
| plain-300 | 301 | letter@start | 19 | 3.0 / 4.2 / 4.3 | p95 | 11 | 4.2 | 20.2 | 0.1 | 19/19 | 2 / 2 | 292 MB | 1.58-1.45 | PASS |
| plain-300 | 301 | letter@middle | 19 | 3.0 / 3.9 / 4.2 | p95 | 11 | 3.9 | 18.7 | 0.1 | 19/19 | 2 / 2 | 292 MB | 1.45-1.5 | PASS |
| plain-300 | 301 | letter@end | 19 | 3.2 / 12.1 / 13.2 | p95 | 11 | 12.1 | 75.0 | 4.9 | 19/19 | 2 / 2 | 292 MB | 1.5-1.38 | MISS: p95 12.1 ms > 11 ms |
| plain-300 | 301 | sentence@middle | 19 | 3.1 / 3.7 / 7.3 | p95 | 11 | 3.7 | 127.3 | 0.1 | 19/19 | 65 / 65 | 292 MB | 1.38-1.63 | PASS |
| plain-300 | 301 | typing@50ms | 39 | 2.7 / 3.0 / 3.4 | p95 | 11 | - | - | 0.1 | - | - | 292 MB | 1.63-1.5 | PASS |
| plain-300 | 301 | typing@60ms | 39 | 2.7 / 3.0 / 3.1 | p95 | 11 | - | - | 0.1 | - | - | 292 MB | 1.5-1.5 | PASS |
| plain-300 | 301 | typing@80ms | 39 | 2.8 / 3.0 / 3.0 | p95 | 11 | - | - | 0.1 | - | - | 292 MB | 1.5-1.46 | PASS |
| plain-300 | 301 | typing@100ms | 39 | 2.8 / 3.6 / 3.8 | p95 | 11 | - | - | 0.2 | - | - | 292 MB | 1.46-1.46 | PASS |
| plain-300 | 301 | typing@150ms | 39 | 2.9 / 3.5 / 5.2 | p95 | 11 | - | - | 0.1 | - | - | 292 MB | 1.46-1.54 | PASS |
| plain-300 | 301 | preamble | 13 | 34.6 / 36.4 / 43.8 | p95 | 400 | 36.4 | 408.6 | 1.0 | 0/13 | 301 / 301 | 292 MB | 1.54-1.53 | PASS |
| plain-300 | 301 | reopen: host's share, cold (spawn -> page 1) | 12 | 56.8 / 58.1 / 62.5 | p95 | (100, app) | - | - | - | - | - | 292 MB | 1.52-1.57 | report-only |
| plain-300 | 301 | reopen: host already listening (not counted, decision 8) | 12 | 17.1 / 17.6 / 18.6 | p95 | (100, app) | - | - | - | - | - | 292 MB | 1.52-1.57 | report-only |
| full-300 | 299 | split@middle | 19 | 7.1 / 8.3 / 8.7 | p95 | 11 | 8.3 | 186.3 | 0.2 | 19/19 | 33 / 33 | 447 MB | 1.52-1.4 | PASS |
| full-300 | 299 | letter@start | 19 | 5.7 / 6.7 / 6.9 | p95 | 11 | 6.7 | 33.7 | 0.2 | 19/19 | 2 / 2 | 447 MB | 1.4-1.88 | PASS |
| full-300 | 299 | letter@middle | 19 | 6.6 / 7.8 / 8.8 | p95 | 11 | 7.8 | 32.9 | 0.2 | 19/19 | 2 / 2 | 447 MB | 1.88-1.81 | PASS |
| full-300 | 299 | letter@end | 19 | 7.3 / 8.9 / 8.9 | p95 | 11 | 4.5 | 35.3 | 0.1 | 19/19 | 3 / 3 | 447 MB | 1.81-1.61 | PASS |
| full-300 | 299 | sentence@middle | 19 | 7.0 / 8.4 / 8.7 | p95 | 11 | 8.4 | 187.8 | 0.2 | 19/19 | 33 / 33 | 447 MB | 1.61-1.64 | PASS |
| full-300 | 299 | newline@middle | 19 | 7.1 / 7.7 / 7.9 | p95 | 11 | 7.7 | 29.1 | 0.2 | 19/19 | 2 / 2 | 447 MB | 1.64-6.92 | PASS |
| full-300 | 299 | typing@50ms | 39 | 6.0 / 6.5 / 6.7 | p95 | 11 | - | - | 0.2 | - | - | 447 MB | 6.92-6.92 | PASS |
| full-300 | 299 | typing@60ms | 39 | 6.1 / 6.6 / 6.7 | p95 | 11 | - | - | 0.2 | - | - | 447 MB | 6.92-6.69 | PASS |
| full-300 | 299 | typing@80ms | 39 | 6.2 / 6.7 / 6.9 | p95 | 11 | - | - | 0.2 | - | - | 447 MB | 6.69-6.69 | PASS |
| full-300 | 299 | typing@100ms | 39 | 6.3 / 6.9 / 7.1 | p95 | 11 | - | - | 0.2 | - | - | 447 MB | 6.69-6.15 | PASS |
| full-300 | 299 | typing@150ms | 39 | 6.5 / 7.3 / 7.4 | p95 | 11 | - | - | 0.2 | - | - | 447 MB | 6.15-5.74 | PASS |
| full-300 | 299 | preamble | 13 | 169.1 / 173.0 / 175.0 | p95 | 400 | 173.0 | 1587.7 | 2.4 | 0/13 | 299 / 299 | 447 MB | 5.74-4.25 | PASS |
| full-300 | 299 | reopen: host's share, cold (spawn -> page 1) | 12 | 98.9 / 123.4 / 365.0 | p95 | (100, app) | - | - | - | - | - | 447 MB | 3.68-4.93 | report-only: p95 123.4 ms > 100 ms |
| full-300 | 299 | reopen: host already listening (not counted, decision 8) | 12 | 57.5 / 58.7 / 307.2 | p95 | (100, app) | - | - | - | - | - | 447 MB | 3.68-4.93 | report-only |
| plain-1000 | 1001 | letter@start | 19 | 3.3 / 4.0 / 4.2 | p95 | 11 | 4.0 | 29.7 | 0.1 | 19/19 | 2 / 2 | 566 MB | 4.35-3.98 | PASS |
| plain-1000 | 1001 | letter@middle | 19 | 4.1 / 12.9 / 22.4 | p95 | 11 | 10.8 | 80.9 | 4.6 | 19/19 | 3 / 3 | 566 MB | 3.98-3.75 | MISS: p95 12.9 ms > 11 ms |
| plain-1000 | 1001 | letter@end | 19 | 3.2 / 4.2 / 6.5 | p95 | 11 | 4.2 | 19.6 | 0.2 | 19/19 | 2 / 2 | 566 MB | 3.75-3.45 | PASS |
| plain-1000 | 1001 | sentence@middle | 19 | 5.0 / 5.7 / 7.1 | p95 | 11 | 4.2 | 3067.2 | 0.2 | 0/19 | 1503 / 1505 | 566 MB | 3.45-4.34 | PASS |
| plain-1000 | 1001 | newline@middle | 19 | 4.8 / 6.0 / 6.5 | p95 | 11 | 4.2 | 30.5 | 0.2 | 19/19 | 3 / 3 | 566 MB | 4.34-4.22 | PASS |
| plain-1000 | 1001 | split@middle | 19 | 5.2 / 6.2 / 9.7 | p95 | 11 | 4.8 | 4015.6 | 0.1 | 0/19 | 1503 / 1505 | 566 MB | 4.22-3.45 | PASS |
| plain-1000 | 1001 | typing@50ms | 39 | 3.8 / 4.3 / 4.6 | p95 | 11 | - | - | 0.1 | - | - | 566 MB | 3.45-3.34 | PASS |
| plain-1000 | 1001 | typing@60ms | 39 | 3.9 / 4.3 / 10.3 | p95 | 11 | - | - | 0.1 | - | - | 566 MB | 3.34-3.34 | PASS |
| plain-1000 | 1001 | typing@80ms | 39 | 4.1 / 4.4 / 4.6 | p95 | 11 | - | - | 0.1 | - | - | 566 MB | 3.34-3.39 | PASS |
| plain-1000 | 1001 | typing@100ms | 39 | 4.0 / 4.3 / 4.8 | p95 | 11 | - | - | 0.1 | - | - | 566 MB | 3.39-3.12 | PASS |
| plain-1000 | 1001 | typing@150ms | 39 | 4.2 / 4.9 / 7.2 | p95 | 11 | - | - | 0.1 | - | - | 566 MB | 3.12-3.03 | PASS |
| plain-1000 | 1001 | preamble | 13 | 39.8 / 41.2 / 41.6 | p95 | 400 | 41.2 | 1283.4 | 1.0 | 0/13 | 1001 / 1001 | 566 MB | 3.03-2.51 | PASS |
| plain-1000 | 1001 | reopen: host's share, cold (spawn -> page 1) | 12 | 58.0 / 60.2 / 61.1 | p95 | (100, app) | - | - | - | - | - | 566 MB | 2.28-2.39 | report-only |
| plain-1000 | 1001 | reopen: host already listening (not counted, decision 8) | 12 | 18.8 / 19.8 / 19.8 | p95 | (100, app) | - | - | - | - | - | 566 MB | 2.28-2.39 | report-only |
| full-1000 | 1002 | letter@middle | 19 | 6.8 / 8.2 / 8.4 | p95 | 11 | 8.2 | 203.1 | 0.1 | 19/19 | 33 / 33 | 817 MB | 2.17-2.29 | PASS |
| full-1000 | 1002 | letter@end | 19 | 7.1 / 8.0 / 8.4 | p95 | 11 | 5.0 | 36.5 | 0.1 | 19/19 | 3 / 3 | 817 MB | 2.29-2.18 | PASS |
| full-1000 | 1002 | sentence@middle | 19 | 6.9 / 8.8 / 9.0 | p95 | 11 | 8.8 | 195.6 | 0.1 | 19/19 | 33 / 33 | 817 MB | 2.18-2.47 | PASS |
| full-1000 | 1002 | newline@middle | 19 | 7.1 / 9.9 / 10.7 | p95 | 11 | 9.9 | 65.1 | 0.6 | 19/19 | 3 / 3 | 817 MB | 2.47-2.33 | PASS |
| full-1000 | 1002 | split@middle | 19 | 7.0 / 7.9 / 8.1 | p95 | 11 | 7.9 | 201.9 | 0.1 | 19/19 | 33 / 33 | 817 MB | 2.33-2.44 | PASS |
| full-1000 | 1002 | letter@start | 19 | 7.0 / 7.6 / 7.9 | p95 | 11 | 7.6 | 53.8 | 0.2 | 19/19 | 2 / 2 | 817 MB | 2.44-2.32 | PASS |
| full-1000 | 1002 | typing@50ms | 39 | 6.7 / 8.2 / 8.2 | p95 | 11 | - | - | 0.3 | - | - | 817 MB | 2.32-2.29 | PASS |
| full-1000 | 1002 | typing@60ms | 39 | 6.7 / 7.3 / 8.2 | p95 | 11 | - | - | 0.4 | - | - | 817 MB | 2.29-2.29 | PASS |
| full-1000 | 1002 | typing@80ms | 39 | 6.9 / 7.5 / 10.1 | p95 | 11 | - | - | 0.4 | - | - | 817 MB | 2.29-2.43 | PASS |
| full-1000 | 1002 | typing@100ms | 39 | 7.0 / 7.4 / 13.9 | p95 | 11 | - | - | 0.4 | - | - | 817 MB | 2.43-2.4 | PASS |
| full-1000 | 1002 | typing@150ms | 39 | 7.4 / 8.7 / 15.4 | p95 | 11 | - | - | 0.5 | - | - | 817 MB | 2.4-2.28 | PASS |
| full-1000 | 1002 | preamble | 13 | 187.9 / 194.1 / 202.9 | p95 | 400 | 194.1 | 5273.7 | 2.6 | 0/13 | 1002 / 1002 | 817 MB | 2.28-2.83 | PASS |
| full-1000 | 1002 | reopen: host's share, cold (spawn -> page 1) | 12 | 108.5 / 118.8 / 119.5 | p95 | (100, app) | - | - | - | - | - | 817 MB | 2.65-3.38 | report-only: p95 118.8 ms > 100 ms |
| full-1000 | 1002 | reopen: host already listening (not counted, decision 8) | 12 | 68.3 / 72.4 / 74.1 | p95 | (100, app) | - | - | - | - | - | 817 MB | 2.65-3.38 | report-only |
| beamer-5 | 5 | letter@end | 19 | 3.9 / 5.7 / 6.8 | p95 | 11 | 5.7 | 29.6 | 1.2 | 19/19 | 2 / 2 | 253 MB | 2.89-2.73 | PASS |
| beamer-5 | 5 | sentence@middle | 19 | 4.0 / 5.4 / 5.7 | p95 | 11 | 5.4 | 20.0 | 1.5 | 19/19 | 2 / 2 | 253 MB | 2.73-2.54 | PASS |
| beamer-5 | 5 | newline@middle | 19 | 3.9 / 5.1 / 5.2 | p95 | 11 | 5.1 | 23.2 | 0.1 | 19/19 | 2 / 3 | 253 MB | 2.54-2.34 | PASS |
| beamer-5 | 5 | split@middle | 19 | 3.9 / 4.3 / 5.2 | p95 | 11 | 4.3 | 44.6 | 0.1 | 19/19 | 3 / 3 | 253 MB | 2.34-2.39 | PASS |
| beamer-5 | 5 | letter@start | 19 | 4.0 / 5.0 / 5.3 | p95 | 11 | 5.0 | 20.0 | 0.2 | 19/19 | 2 / 2 | 253 MB | 2.39-2.26 | PASS |
| beamer-5 | 5 | letter@middle | 19 | 4.1 / 5.4 / 5.9 | p95 | 11 | 5.4 | 19.5 | 0.1 | 19/19 | 2 / 2 | 253 MB | 2.26-2.48 | PASS |
| beamer-5 | 5 | typing@50ms | 39 | 3.7 / 4.6 / 11.4 | p95 | 11 | - | - | 0.1 | - | - | 253 MB | 2.48-2.48 | PASS |
| beamer-5 | 5 | typing@60ms | 39 | 3.7 / 4.1 / 4.2 | p95 | 11 | - | - | 0.1 | - | - | 253 MB | 2.48-2.44 | PASS |
| beamer-5 | 5 | typing@80ms | 39 | 3.8 / 4.4 / 8.2 | p95 | 11 | - | - | 0.1 | - | - | 253 MB | 2.44-2.56 | PASS |
| beamer-5 | 5 | typing@100ms | 39 | 3.9 / 4.3 / 4.6 | p95 | 11 | - | - | 0.1 | - | - | 253 MB | 2.56-2.56 | PASS |
| beamer-5 | 5 | typing@150ms | 39 | 4.0 / 5.0 / 9.6 | p95 | 11 | - | - | 0.2 | - | - | 253 MB | 2.56-2.84 | PASS |
| beamer-5 | 5 | preamble | 13 | 233.2 / 254.8 / 311.4 | p95 | 400 | 254.8 | 291.9 | 22.1 | 0/13 | 5 / 5 | 253 MB | 2.84-2.93 | PASS |
| beamer-5 | 5 | reopen: host's share, cold (spawn -> page 1) | 12 | 97.7 / 113.5 / 122.7 | p95 | (100, app) | - | - | - | - | - | 253 MB | 2.93-3.42 | report-only: p95 113.5 ms > 100 ms |
| beamer-5 | 5 | reopen: host already listening (not counted, decision 8) | 12 | 56.3 / 62.8 / 70.0 | p95 | (100, app) | - | - | - | - | - | 253 MB | 2.93-3.42 | report-only |
| beamer-30 | 58 | sentence@middle | 19 | 8.4 / 9.8 / 10.5 | p95 | 11 | 9.8 | 48.8 | 0.6 | 19/19 | 2 / 2 | 311 MB | 3.22-3.26 | PASS |
| beamer-30 | 58 | newline@middle | 19 | 7.5 / 8.8 / 22.9 | p95 | 11 | 8.8 | 71.0 | 0.1 | 19/19 | 3 / 3 | 311 MB | 3.26-3.16 | PASS |
| beamer-30 | 58 | split@middle | 19 | 8.3 / 11.2 / 11.7 | p95 | 11 | 11.2 | 71.2 | 0.4 | 19/19 | 3 / 3 | 311 MB | 3.16-3.05 | MISS: p95 11.2 ms > 11 ms |
| beamer-30 | 58 | letter@start | 19 | 8.5 / 9.3 / 9.3 | p95 | 11 | 9.3 | 43.6 | 0.2 | 19/19 | 2 / 2 | 311 MB | 3.05-3.05 | PASS |
| beamer-30 | 58 | letter@middle | 19 | 7.8 / 9.4 / 17.3 | p95 | 11 | 9.4 | 31.2 | 0.1 | 19/19 | 2 / 2 | 311 MB | 3.05-6.5 | PASS |
| beamer-30 | 58 | letter@end | 19 | 8.5 / 9.4 / 9.7 | p95 | 11 | 9.4 | 45.6 | 0.2 | 19/19 | 2 / 2 | 311 MB | 6.5-6.14 | PASS |
| beamer-30 | 58 | typing@50ms | 39 | 8.3 / 11.3 / 13.4 | p95 | 11 | - | - | 0.3 | - | - | 311 MB | 6.14-6.14 | MISS: p95 11.3 ms > 11 ms |
| beamer-30 | 58 | typing@60ms | 39 | 7.0 / 8.2 / 9.0 | p95 | 11 | - | - | 0.1 | - | - | 311 MB | 6.14-6.05 | PASS |
| beamer-30 | 58 | typing@80ms | 39 | 7.1 / 9.1 / 12.8 | p95 | 11 | - | - | 2.2 | - | - | 311 MB | 6.05-6.05 | PASS |
| beamer-30 | 58 | typing@100ms | 39 | 7.3 / 8.1 / 9.1 | p95 | 11 | - | - | 0.1 | - | - | 311 MB | 6.05-5.88 | PASS |
| beamer-30 | 58 | typing@150ms | 39 | 7.7 / 9.3 / 11.6 | p95 | 11 | - | - | 0.2 | - | - | 311 MB | 5.88-5.89 | PASS |
| beamer-30 | 58 | preamble | 13 | 253.1 / 261.5 / 263.0 | p95 | 400 | 261.5 | 1127.7 | 5.7 | 0/13 | 58 / 58 | 311 MB | 5.89-5.02 | PASS |
| beamer-30 | 58 | reopen: host's share, cold (spawn -> page 1) | 12 | 110.0 / 117.5 / 119.5 | p95 | (100, app) | - | - | - | - | - | 311 MB | 4.51-4.87 | report-only: p95 117.5 ms > 100 ms |
| beamer-30 | 58 | reopen: host already listening (not counted, decision 8) | 12 | 69.1 / 74.8 / 77.5 | p95 | (100, app) | - | - | - | - | - | 311 MB | 4.51-4.87 | report-only |

ms: COMPILE written to the watched page's PAGE frame read on the socket (preamble, reopen: page 1); typing@: the keystroke sent to the watched page from its own compile or a later one. off-CPU: of the host's time to that page, what its engine thread spent descheduled (wall minus queue minus thread CPU): load or a CPU quota, not the engine. The first sample of each phase is a warm-up, left out; rows with n < 12 are gated on their maximum. No noise margin: a row passes only when it meets its target.
