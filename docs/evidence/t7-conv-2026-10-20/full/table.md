**NON-REFERENCE run**: power source unknown at start; power source unknown at end; CPU quota of flashtex-agents.slice throttled 4059 of 16419 periods (717.9 s stopped)

| doc | pages | edit | n | ms p50 / p95 / max | gated on | target | first changed page p95 | DONE p95 ms | off-CPU p95 ms | converged | re-typeset pages p50 / max | host RSS peak | load1 | verdict |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| plain-10 | 10 | letter@start | 19 | 6.8 / 11.7 / 11.9 | p95 | 11 | 11.7 | 34.9 | 0.1 | 19/19 | 2 / 2 | 159 MB | 2.57-2.65 | MISS: p95 11.7 ms > 11 ms |
| plain-10 | 10 | letter@middle | 19 | 5.2 / 5.8 / 5.9 | p95 | 11 | 3.0 | 26.8 | 0.0 | 19/19 | 3 / 3 | 159 MB | 2.65-2.51 | PASS |
| plain-10 | 10 | letter@end | 19 | 3.3 / 3.9 / 4.1 | p95 | 11 | 3.9 | 14.4 | 0.0 | 0/19 | 1 / 1 | 159 MB | 2.51-2.39 | PASS |
| plain-10 | 10 | sentence@middle | 19 | 5.0 / 5.4 / 5.8 | p95 | 11 | 2.5 | 32.6 | 0.0 | 0/19 | 6 / 6 | 159 MB | 2.39-2.28 | PASS |
| plain-10 | 10 | newline@middle | 19 | 5.1 / 5.6 / 5.7 | p95 | 11 | 2.8 | 31.1 | 0.0 | 19/19 | 3 / 3 | 159 MB | 2.28-2.16 | PASS |
| plain-10 | 10 | split@middle | 19 | 5.3 / 5.6 / 5.6 | p95 | 11 | 2.8 | 32.2 | 0.0 | 0/19 | 6 / 6 | 159 MB | 2.16-2.07 | PASS |
| plain-10 | 10 | typing@50ms | 39 | 4.6 / 4.8 / 5.1 | p95 | 11 | - | - | 0.0 | - | - | 159 MB | 2.07-2.07 | PASS |
| plain-10 | 10 | typing@60ms | 39 | 4.9 / 5.4 / 5.8 | p95 | 11 | - | - | 0.0 | - | - | 159 MB | 2.07-2.14 | PASS |
| plain-10 | 10 | typing@80ms | 39 | 4.9 / 5.4 / 5.8 | p95 | 11 | - | - | 0.0 | - | - | 159 MB | 2.14-2.13 | PASS |
| plain-10 | 10 | typing@100ms | 39 | 35.2 / 36.0 / 38.7 | p95 | 11 | - | - | 0.0 | - | - | 159 MB | 2.13-2.13 | MISS: p95 36.0 ms > 11 ms |
| plain-10 | 10 | typing@150ms | 39 | 5.7 / 14.7 / 16.2 | p95 | 11 | - | - | 0.0 | - | - | 159 MB | 2.13-2.57 | MISS: p95 14.7 ms > 11 ms |
| plain-10 | 10 | preamble | 13 | 105.2 / 116.8 / 153.5 | p95 | 400 | 116.8 | 181.1 | 50.4 | 0/13 | 10 / 10 | 159 MB | 2.57-2.85 | PASS |
| plain-10 | 10 | reopen: host's share, cold (spawn -> page 1) | 12 | 189.6 / 192.6 / 196.4 | p95 | (100, app) | - | - | - | - | - | 159 MB | 2.85-3.25 | report-only: p95 192.6 ms > 100 ms |
| plain-10 | 10 | reopen: host already listening (not counted, decision 8) | 12 | 22.9 / 74.4 / 78.8 | p95 | (100, app) | - | - | - | - | - | 159 MB | 2.85-3.25 | report-only |
| full-10 | 11 | letter@middle | 19 | 10.5 / 11.1 / 16.6 | p95 | 11 | 4.8 | 103.7 | 0.1 | 19/19 | 3 / 3 | 191 MB | 3.25-3.6 | MISS: p95 11.1 ms > 11 ms |
| full-10 | 11 | letter@end | 19 | 13.9 / 59.5 / 60.2 | p95 | 11 | 8.5 | 99.1 | 0.1 | 19/19 | 3 / 3 | 191 MB | 3.6-3.47 | MISS: p95 59.5 ms > 11 ms |
| full-10 | 11 | sentence@middle | 19 | 9.4 / 10.8 / 11.3 | p95 | 11 | 4.5 | 60.0 | 0.0 | 0/19 | 7 / 7 | 191 MB | 3.47-3.09 | PASS |
| full-10 | 11 | newline@middle | 19 | 9.5 / 11.0 / 32.2 | p95 | 11 | 4.9 | 55.2 | 0.0 | 19/19 | 3 / 3 | 191 MB | 3.09-3.16 | MISS: p95 11.0 ms > 11 ms |
| full-10 | 11 | split@middle | 19 | 10.3 / 10.8 / 11.1 | p95 | 11 | 4.5 | 49.2 | 0.0 | 19/19 | 3 / 3 | 191 MB | 3.16-3.06 | PASS |
| full-10 | 11 | letter@start | 19 | 15.3 / 60.7 / 67.1 | p95 | 11 | 60.7 | 112.1 | 44.5 | 19/19 | 2 / 2 | 191 MB | 3.06-3.21 | MISS: p95 60.7 ms > 11 ms |
| full-10 | 11 | typing@50ms | 39 | 14.7 / 62.5 / 63.7 | p95 | 11 | - | - | 0.0 | - | - | 191 MB | 3.21-3.44 | MISS: p95 62.5 ms > 11 ms; 19 of 39 keystrokes not painted by their own compile |
| full-10 | 11 | typing@60ms | 39 | 25.9 / 62.4 / 71.2 | p95 | 11 | - | - | 51.7 | - | - | 191 MB | 3.44-3.44 | MISS: p95 62.4 ms > 11 ms; 1 of 39 keystrokes not painted by their own compile |
| full-10 | 11 | typing@80ms | 39 | 25.1 / 62.8 / 67.0 | p95 | 11 | - | - | 51.1 | - | - | 191 MB | 3.44-3.64 | MISS: p95 62.8 ms > 11 ms |
| full-10 | 11 | typing@100ms | 39 | 10.8 / 15.5 / 21.1 | p95 | 11 | - | - | 0.9 | - | - | 191 MB | 3.64-3.64 | MISS: p95 15.5 ms > 11 ms |
| full-10 | 11 | typing@150ms | 39 | 26.2 / 26.7 / 27.1 | p95 | 11 | - | - | 0.0 | - | - | 191 MB | 3.64-4.0 | MISS: p95 26.7 ms > 11 ms |
| full-10 | 11 | preamble | 13 | 496.6 / 500.4 / 501.4 | p95 | 400 | 500.4 | 652.3 | 244.1 | 0/13 | 11 / 11 | 191 MB | 4.0-4.23 | MISS: p95 500.4 ms > 400 ms |
| full-10 | 11 | reopen: host's share, cold (spawn -> page 1) | 12 | 141.2 / 287.6 / 297.3 | p95 | (100, app) | - | - | - | - | - | 191 MB | 4.2-4.21 | report-only: p95 287.6 ms > 100 ms |
| full-10 | 11 | reopen: host already listening (not counted, decision 8) | 12 | 72.2 / 123.6 / 174.4 | p95 | (100, app) | - | - | - | - | - | 191 MB | 4.2-4.21 | report-only: p95 123.6 ms > 100 ms |
| plain-100 | 100 | letter@end | 19 | 5.3 / 5.9 / 6.1 | p95 | 11 | 3.0 | 25.7 | 0.0 | 19/19 | 3 / 3 | 181 MB | 4.2-3.78 | PASS |
| plain-100 | 100 | sentence@middle | 19 | 3.8 / 4.3 / 4.4 | p95 | 11 | 4.3 | 179.2 | 0.0 | 0/19 | 50 / 50 | 181 MB | 3.78-3.35 | PASS |
| plain-100 | 100 | newline@middle | 19 | 4.1 / 4.5 / 4.7 | p95 | 11 | 4.5 | 23.7 | 0.0 | 19/19 | 2 / 2 | 181 MB | 3.35-3.16 | PASS |
| plain-100 | 100 | split@middle | 19 | 4.1 / 4.6 / 4.8 | p95 | 11 | 4.6 | 26.8 | 0.0 | 19/19 | 2 / 2 | 181 MB | 3.16-2.99 | PASS |
| plain-100 | 100 | letter@start | 19 | 3.9 / 4.3 / 4.4 | p95 | 11 | 4.3 | 30.4 | 0.0 | 19/19 | 2 / 2 | 181 MB | 2.99-2.76 | PASS |
| plain-100 | 100 | letter@middle | 19 | 4.0 / 4.7 / 5.0 | p95 | 11 | 4.7 | 28.5 | 0.0 | 19/19 | 2 / 2 | 181 MB | 2.76-2.61 | PASS |
| plain-100 | 100 | typing@50ms | 39 | 3.7 / 3.8 / 3.9 | p95 | 11 | - | - | 0.0 | - | - | 181 MB | 2.61-2.61 | PASS |
| plain-100 | 100 | typing@60ms | 39 | 3.6 / 4.1 / 5.0 | p95 | 11 | - | - | 0.0 | - | - | 181 MB | 2.61-2.57 | PASS |
| plain-100 | 100 | typing@80ms | 39 | 3.7 / 3.8 / 4.4 | p95 | 11 | - | - | 0.0 | - | - | 181 MB | 2.57-2.44 | PASS |
| plain-100 | 100 | typing@100ms | 39 | 3.7 / 4.0 / 4.1 | p95 | 11 | - | - | 0.0 | - | - | 181 MB | 2.44-2.44 | PASS |
| plain-100 | 100 | typing@150ms | 39 | 3.8 / 4.3 / 4.5 | p95 | 11 | - | - | 0.0 | - | - | 181 MB | 2.44-2.22 | PASS |
| plain-100 | 100 | preamble | 13 | 59.5 / 60.5 / 61.7 | p95 | 400 | 60.5 | 324.6 | 0.3 | 0/13 | 100 / 100 | 181 MB | 2.22-2.12 | PASS |
| plain-100 | 100 | reopen: host's share, cold (spawn -> page 1) | 12 | 90.2 / 92.0 / 93.4 | p95 | (100, app) | - | - | - | - | - | 181 MB | 2.08-2.19 | report-only |
| plain-100 | 100 | reopen: host already listening (not counted, decision 8) | 12 | 21.7 / 22.2 / 23.0 | p95 | (100, app) | - | - | - | - | - | 181 MB | 2.08-2.19 | report-only |
| full-100 | 101 | sentence@middle | 19 | 7.5 / 8.7 / 11.4 | p95 | 11 | 8.7 | 45.8 | 0.1 | 19/19 | 2 / 2 | 224 MB | 2.08-1.99 | PASS |
| full-100 | 101 | newline@middle | 19 | 8.4 / 8.8 / 8.9 | p95 | 11 | 8.8 | 40.5 | 0.1 | 19/19 | 2 / 2 | 224 MB | 1.99-2.0 | PASS |
| full-100 | 101 | split@middle | 19 | 8.3 / 8.7 / 8.8 | p95 | 11 | 8.7 | 93.4 | 0.0 | 19/19 | 2 / 2 | 224 MB | 2.0-1.84 | PASS |
| full-100 | 101 | letter@start | 19 | 8.5 / 9.2 / 10.0 | p95 | 11 | 9.2 | 104.3 | 0.1 | 19/19 | 2 / 2 | 224 MB | 1.84-1.69 | PASS |
| full-100 | 101 | letter@middle | 19 | 8.1 / 12.2 / 35.7 | p95 | 11 | 12.2 | 109.7 | 0.1 | 19/19 | 2 / 2 | 224 MB | 1.69-1.43 | MISS: p95 12.2 ms > 11 ms |
| full-100 | 101 | letter@end | 19 | 11.1 / 11.8 / 12.2 | p95 | 11 | 4.8 | 103.6 | 0.0 | 19/19 | 3 / 3 | 224 MB | 1.43-1.31 | MISS: p95 11.8 ms > 11 ms |
| full-100 | 101 | typing@50ms | 39 | 17.3 / 62.1 / 63.5 | p95 | 11 | - | - | 0.0 | - | - | 224 MB | 1.31-1.21 | MISS: p95 62.1 ms > 11 ms; 18 of 39 keystrokes not painted by their own compile |
| full-100 | 101 | typing@60ms | 39 | 30.1 / 60.5 / 61.0 | p95 | 11 | - | - | 51.4 | - | - | 224 MB | 1.21-1.21 | MISS: p95 60.5 ms > 11 ms |
| full-100 | 101 | typing@80ms | 39 | 15.1 / 55.0 / 56.6 | p95 | 11 | - | - | 1.0 | - | - | 224 MB | 1.21-1.11 | MISS: p95 55.0 ms > 11 ms |
| full-100 | 101 | typing@100ms | 39 | 7.2 / 9.6 / 10.3 | p95 | 11 | - | - | 0.9 | - | - | 224 MB | 1.11-1.02 | PASS |
| full-100 | 101 | typing@150ms | 39 | 7.8 / 55.3 / 57.3 | p95 | 11 | - | - | 47.1 | - | - | 224 MB | 1.02-0.94 | MISS: p95 55.3 ms > 11 ms |
| full-100 | 101 | preamble | 13 | 270.4 / 274.1 / 274.5 | p95 | 400 | 274.1 | 1045.8 | 1.3 | 0/13 | 101 / 101 | 224 MB | 0.94-1.32 | PASS |
| full-100 | 101 | reopen: host's share, cold (spawn -> page 1) | 12 | 289.6 / 294.5 / 295.1 | p95 | (100, app) | - | - | - | - | - | 224 MB | 1.04-1.46 | report-only: p95 294.5 ms > 100 ms |
| full-100 | 101 | reopen: host already listening (not counted, decision 8) | 12 | 125.8 / 172.6 / 179.0 | p95 | (100, app) | - | - | - | - | - | 224 MB | 1.04-1.46 | report-only: p95 172.6 ms > 100 ms |
| plain-300 | 301 | newline@middle | 19 | 4.8 / 6.6 / 48.0 | p95 | 11 | 6.6 | 88.0 | 0.1 | 19/19 | 4 / 4 | 258 MB | 0.96-0.81 | PASS |
| plain-300 | 301 | split@middle | 19 | 4.9 / 5.5 / 5.7 | p95 | 11 | 5.5 | 111.7 | 0.0 | 19/19 | 9 / 9 | 258 MB | 0.81-0.99 | PASS |
| plain-300 | 301 | letter@start | 19 | 4.2 / 4.8 / 4.9 | p95 | 11 | 4.8 | 35.2 | 0.0 | 19/19 | 2 / 2 | 258 MB | 0.99-0.99 | PASS |
| plain-300 | 301 | letter@middle | 19 | 4.4 / 4.9 / 5.0 | p95 | 11 | 4.9 | 34.6 | 0.0 | 19/19 | 2 / 2 | 258 MB | 0.99-1.08 | PASS |
| plain-300 | 301 | letter@end | 19 | 4.0 / 4.7 / 11.1 | p95 | 11 | 4.7 | 30.3 | 0.1 | 19/19 | 2 / 2 | 258 MB | 1.08-1.06 | PASS |
| plain-300 | 301 | sentence@middle | 19 | 4.5 / 4.9 / 5.1 | p95 | 11 | 4.9 | 220.5 | 0.0 | 19/19 | 65 / 65 | 258 MB | 1.06-1.13 | PASS |
| plain-300 | 301 | typing@50ms | 39 | 3.9 / 4.4 / 4.5 | p95 | 11 | - | - | 0.0 | - | - | 258 MB | 1.13-1.13 | PASS |
| plain-300 | 301 | typing@60ms | 39 | 3.9 / 4.2 / 4.4 | p95 | 11 | - | - | 0.0 | - | - | 258 MB | 1.13-1.28 | PASS |
| plain-300 | 301 | typing@80ms | 39 | 3.9 / 4.8 / 4.9 | p95 | 11 | - | - | 0.0 | - | - | 258 MB | 1.28-1.28 | PASS |
| plain-300 | 301 | typing@100ms | 39 | 3.9 / 4.3 / 4.8 | p95 | 11 | - | - | 0.0 | - | - | 258 MB | 1.28-1.25 | PASS |
| plain-300 | 301 | typing@150ms | 39 | 4.2 / 4.8 / 4.9 | p95 | 11 | - | - | 0.0 | - | - | 258 MB | 1.25-1.31 | PASS |
| plain-300 | 301 | preamble | 13 | 66.5 / 114.6 / 157.0 | p95 | 400 | 114.6 | 1877.6 | 50.9 | 0/13 | 301 / 301 | 258 MB | 1.31-1.31 | PASS |
| plain-300 | 301 | reopen: host's share, cold (spawn -> page 1) | 12 | 188.8 / 194.9 / 198.8 | p95 | (100, app) | - | - | - | - | - | 258 MB | 0.94-1.74 | report-only: p95 194.9 ms > 100 ms |
| plain-300 | 301 | reopen: host already listening (not counted, decision 8) | 12 | 47.6 / 75.9 / 76.5 | p95 | (100, app) | - | - | - | - | - | 258 MB | 0.94-1.74 | report-only |
| full-300 | 299 | split@middle | 19 | 9.8 / 10.5 / 10.7 | p95 | 11 | 10.5 | 306.3 | 0.1 | 19/19 | 33 / 33 | 339 MB | 1.74-1.73 | PASS |
| full-300 | 299 | letter@start | 19 | 8.0 / 8.6 / 8.6 | p95 | 11 | 8.6 | 57.7 | 0.1 | 19/19 | 2 / 2 | 339 MB | 1.73-1.68 | PASS |
| full-300 | 299 | letter@middle | 19 | 9.7 / 10.4 / 10.5 | p95 | 11 | 10.4 | 53.3 | 0.1 | 19/19 | 2 / 2 | 339 MB | 1.68-1.73 | PASS |
| full-300 | 299 | letter@end | 19 | 10.8 / 11.7 / 11.9 | p95 | 11 | 4.9 | 53.8 | 0.0 | 19/19 | 3 / 3 | 339 MB | 1.73-1.83 | MISS: p95 11.7 ms > 11 ms |
| full-300 | 299 | sentence@middle | 19 | 10.0 / 10.3 / 10.4 | p95 | 11 | 10.3 | 307.7 | 0.1 | 19/19 | 33 / 33 | 339 MB | 1.83-1.78 | PASS |
| full-300 | 299 | newline@middle | 19 | 10.1 / 10.6 / 10.7 | p95 | 11 | 10.6 | 45.9 | 0.1 | 19/19 | 2 / 2 | 339 MB | 1.78-1.81 | PASS |
| full-300 | 299 | typing@50ms | 39 | 12.8 / 19.7 / 20.3 | p95 | 11 | - | - | 4.6 | - | - | 339 MB | 1.81-1.81 | MISS: p95 19.7 ms > 11 ms |
| full-300 | 299 | typing@60ms | 39 | 9.1 / 12.7 / 13.2 | p95 | 11 | - | - | 1.6 | - | - | 339 MB | 1.81-1.91 | MISS: p95 12.7 ms > 11 ms |
| full-300 | 299 | typing@80ms | 39 | 9.3 / 9.8 / 9.9 | p95 | 11 | - | - | 0.1 | - | - | 339 MB | 1.91-1.91 | PASS |
| full-300 | 299 | typing@100ms | 39 | 9.4 / 9.8 / 12.5 | p95 | 11 | - | - | 0.0 | - | - | 339 MB | 1.91-2.0 | PASS |
| full-300 | 299 | typing@150ms | 39 | 9.8 / 10.6 / 11.0 | p95 | 11 | - | - | 0.1 | - | - | 339 MB | 2.0-2.08 | PASS |
| full-300 | 299 | preamble | 13 | 277.1 / 279.5 / 286.0 | p95 | 400 | 279.5 | 2646.0 | 1.3 | 0/13 | 299 / 299 | 339 MB | 2.08-2.62 | PASS |
| full-300 | 299 | reopen: host's share, cold (spawn -> page 1) | 12 | 146.2 / 150.3 / 150.4 | p95 | (100, app) | - | - | - | - | - | 339 MB | 2.62-2.92 | report-only: p95 150.3 ms > 100 ms |
| full-300 | 299 | reopen: host already listening (not counted, decision 8) | 12 | 77.8 / 80.0 / 80.2 | p95 | (100, app) | - | - | - | - | - | 339 MB | 2.62-2.92 | report-only |
| plain-1000 | 1001 | letter@start | 19 | 5.5 / 6.5 / 6.7 | p95 | 11 | 6.5 | 64.3 | 0.0 | 19/19 | 2 / 2 | 531 MB | 2.92-2.86 | PASS |
| plain-1000 | 1001 | letter@middle | 19 | 6.7 / 8.5 / 8.8 | p95 | 11 | 5.0 | 49.3 | 0.0 | 19/19 | 3 / 3 | 531 MB | 2.86-2.73 | PASS |
| plain-1000 | 1001 | letter@end | 19 | 5.0 / 6.1 / 6.4 | p95 | 11 | 6.1 | 32.4 | 0.0 | 19/19 | 2 / 2 | 531 MB | 2.73-2.67 | PASS |
| plain-1000 | 1001 | sentence@middle | 19 | 6.9 / 8.3 / 8.4 | p95 | 11 | 5.5 | 6447.4 | 0.0 | 0/19 | 1503 / 1505 | 531 MB | 2.67-2.93 | PASS |
| plain-1000 | 1001 | newline@middle | 19 | 8.0 / 8.6 / 8.9 | p95 | 11 | 5.9 | 48.1 | 0.0 | 19/19 | 3 / 3 | 531 MB | 2.93-2.77 | PASS |
| plain-1000 | 1001 | split@middle | 19 | 7.7 / 8.4 / 9.2 | p95 | 11 | 5.8 | 10060.9 | 0.0 | 0/19 | 1503 / 1505 | 531 MB | 2.77-1.51 | PASS |
| plain-1000 | 1001 | typing@50ms | 39 | 10.1 / 10.7 / 11.0 | p95 | 11 | - | - | 2.1 | - | - | 531 MB | 1.51-1.55 | PASS |
| plain-1000 | 1001 | typing@60ms | 39 | 6.4 / 9.7 / 10.5 | p95 | 11 | - | - | 2.0 | - | - | 531 MB | 1.55-1.55 | PASS |
| plain-1000 | 1001 | typing@80ms | 39 | 6.5 / 6.9 / 7.3 | p95 | 11 | - | - | 0.0 | - | - | 531 MB | 1.55-1.59 | PASS |
| plain-1000 | 1001 | typing@100ms | 39 | 6.6 / 7.0 / 7.4 | p95 | 11 | - | - | 0.0 | - | - | 531 MB | 1.59-1.54 | PASS |
| plain-1000 | 1001 | typing@150ms | 39 | 6.7 / 7.3 / 7.6 | p95 | 11 | - | - | 0.0 | - | - | 531 MB | 1.54-1.58 | PASS |
| plain-1000 | 1001 | preamble | 13 | 82.5 / 85.4 / 85.7 | p95 | 400 | 85.4 | 2921.9 | 0.6 | 0/13 | 1001 / 1001 | 531 MB | 1.58-2.25 | PASS |
| plain-1000 | 1001 | reopen: host's share, cold (spawn -> page 1) | 12 | 98.5 / 99.6 / 101.3 | p95 | (100, app) | - | - | - | - | - | 531 MB | 2.28-2.68 | report-only |
| plain-1000 | 1001 | reopen: host already listening (not counted, decision 8) | 12 | 28.5 / 28.9 / 29.5 | p95 | (100, app) | - | - | - | - | - | 531 MB | 2.28-2.68 | report-only |
| full-1000 | 1002 | letter@middle | 19 | 10.9 / 11.8 / 15.0 | p95 | 11 | 11.8 | 346.1 | 0.1 | 19/19 | 33 / 33 | 627 MB | 2.68-2.44 | MISS: p95 11.8 ms > 11 ms |
| full-1000 | 1002 | letter@end | 19 | 10.9 / 11.6 / 19.4 | p95 | 11 | 6.3 | 99.6 | 0.0 | 19/19 | 3 / 3 | 627 MB | 2.44-2.48 | MISS: p95 11.6 ms > 11 ms |
| full-1000 | 1002 | sentence@middle | 19 | 10.7 / 36.8 / 45.4 | p95 | 11 | 36.8 | 579.0 | 25.2 | 19/19 | 33 / 33 | 627 MB | 2.48-3.35 | MISS: p95 36.8 ms > 11 ms |
| full-1000 | 1002 | newline@middle | 19 | 11.9 / 17.4 / 47.9 | p95 | 11 | 17.4 | 161.8 | 5.5 | 19/19 | 3 / 3 | 627 MB | 3.35-3.64 | MISS: p95 17.4 ms > 11 ms |
| full-1000 | 1002 | split@middle | 19 | 12.3 / 13.4 / 44.5 | p95 | 11 | 13.4 | 441.4 | 0.1 | 19/19 | 33 / 33 | 627 MB | 3.64-3.69 | MISS: p95 13.4 ms > 11 ms |
| full-1000 | 1002 | letter@start | 19 | 12.0 / 13.1 / 17.5 | p95 | 11 | 13.1 | 100.5 | 0.1 | 19/19 | 2 / 2 | 627 MB | 3.69-3.59 | MISS: p95 13.1 ms > 11 ms |
| full-1000 | 1002 | typing@50ms | 39 | 14.4 / 16.1 / 43.5 | p95 | 11 | - | - | 2.9 | - | - | 627 MB | 3.59-3.59 | MISS: p95 16.1 ms > 11 ms |
| full-1000 | 1002 | typing@60ms | 39 | 14.0 / 16.1 / 17.1 | p95 | 11 | - | - | 2.8 | - | - | 627 MB | 3.59-3.54 | MISS: p95 16.1 ms > 11 ms |
| full-1000 | 1002 | typing@80ms | 39 | 12.5 / 13.8 / 17.3 | p95 | 11 | - | - | 0.5 | - | - | 627 MB | 3.54-3.54 | MISS: p95 13.8 ms > 11 ms |
| full-1000 | 1002 | typing@100ms | 39 | 12.8 / 15.0 / 15.6 | p95 | 11 | - | - | 0.6 | - | - | 627 MB | 3.54-3.58 | MISS: p95 15.0 ms > 11 ms |
| full-1000 | 1002 | typing@150ms | 39 | 12.9 / 14.2 / 14.7 | p95 | 11 | - | - | 0.7 | - | - | 627 MB | 3.58-3.53 | MISS: p95 14.2 ms > 11 ms |
| full-1000 | 1002 | preamble | 13 | 309.3 / 312.6 / 313.3 | p95 | 400 | 312.6 | 8075.2 | 1.5 | 0/13 | 1002 / 1002 | 627 MB | 3.53-3.47 | PASS |
| full-1000 | 1002 | reopen: host's share, cold (spawn -> page 1) | 12 | 162.0 / 167.3 / 287.3 | p95 | (100, app) | - | - | - | - | - | 627 MB | 1.67-3.25 | report-only: p95 167.3 ms > 100 ms |
| full-1000 | 1002 | reopen: host already listening (not counted, decision 8) | 12 | 91.3 / 95.7 / 175.8 | p95 | (100, app) | - | - | - | - | - | 627 MB | 1.67-3.25 | report-only |
| beamer-5 | 5 | letter@end | 19 | 16.6 / 48.8 / 48.9 | p95 | 11 | 48.8 | 95.3 | 32.2 | 19/19 | 2 / 2 | 213 MB | 1.62-1.45 | MISS: p95 48.8 ms > 11 ms (no baseline row) |
| beamer-5 | 5 | sentence@middle | 19 | 16.9 / 47.3 / 48.9 | p95 | 11 | 47.3 | 91.9 | 30.2 | 19/19 | 2 / 2 | 213 MB | 1.45-1.31 | MISS: p95 47.3 ms > 11 ms (no baseline row) |
| beamer-5 | 5 | newline@middle | 19 | 16.1 / 52.0 / 52.5 | p95 | 11 | 52.0 | 91.4 | 35.2 | 19/19 | 2 / 2 | 213 MB | 1.31-1.28 | MISS: p95 52.0 ms > 11 ms (no baseline row) |
| beamer-5 | 5 | split@middle | 19 | 15.3 / 16.4 / 16.4 | p95 | 11 | 16.4 | 59.6 | 0.1 | 19/19 | 2 / 2 | 213 MB | 1.28-1.26 | MISS: p95 16.4 ms > 11 ms (no baseline row) |
| beamer-5 | 5 | letter@start | 19 | 16.2 / 17.8 / 17.9 | p95 | 11 | 17.8 | 53.3 | 0.1 | 19/19 | 2 / 2 | 213 MB | 1.26-1.22 | MISS: p95 17.8 ms > 11 ms (no baseline row) |
| beamer-5 | 5 | letter@middle | 19 | 15.7 / 17.5 / 17.7 | p95 | 11 | 17.5 | 55.1 | 0.1 | 19/19 | 2 / 2 | 213 MB | 1.22-1.28 | MISS: p95 17.5 ms > 11 ms (no baseline row) |
| beamer-5 | 5 | typing@50ms | 39 | 17.3 / 19.3 / 20.1 | p95 | 11 | - | - | 0.6 | - | - | 213 MB | 1.28-1.34 | MISS: p95 19.3 ms > 11 ms |
| beamer-5 | 5 | typing@60ms | 39 | 15.1 / 15.3 / 17.2 | p95 | 11 | - | - | 0.1 | - | - | 213 MB | 1.34-1.34 | MISS: p95 15.3 ms > 11 ms |
| beamer-5 | 5 | typing@80ms | 39 | 15.4 / 16.7 / 20.1 | p95 | 11 | - | - | 0.1 | - | - | 213 MB | 1.34-1.39 | MISS: p95 16.7 ms > 11 ms |
| beamer-5 | 5 | typing@100ms | 39 | 15.8 / 16.7 / 17.0 | p95 | 11 | - | - | 0.1 | - | - | 213 MB | 1.39-1.44 | MISS: p95 16.7 ms > 11 ms |
| beamer-5 | 5 | typing@150ms | 39 | 15.6 / 16.4 / 18.2 | p95 | 11 | - | - | 0.1 | - | - | 213 MB | 1.44-1.41 | MISS: p95 16.4 ms > 11 ms |
| beamer-5 | 5 | preamble | 13 | 373.6 / 378.8 / 385.4 | p95 | 400 | 378.8 | 447.1 | 1.7 | 0/13 | 5 / 5 | 213 MB | 1.41-1.42 | PASS (no baseline row) |
| beamer-5 | 5 | reopen: host's share, cold (spawn -> page 1) | 12 | 137.6 / 140.3 / 144.7 | p95 | (100, app) | - | - | - | - | - | 213 MB | 1.36-1.42 | report-only: p95 140.3 ms > 100 ms |
| beamer-5 | 5 | reopen: host already listening (not counted, decision 8) | 12 | 69.3 / 70.2 / 72.0 | p95 | (100, app) | - | - | - | - | - | 213 MB | 1.36-1.42 | report-only |
| beamer-30 | 58 | sentence@middle | 19 | 35.1 / 36.4 / 38.8 | p95 | 11 | 36.4 | 89.6 | 0.2 | 19/19 | 2 / 2 | 210 MB | 1.36-1.38 | MISS: p95 36.4 ms > 11 ms (no baseline row) |
| beamer-30 | 58 | newline@middle | 19 | 34.2 / 35.3 / 35.7 | p95 | 11 | 35.3 | 108.7 | 0.2 | 19/19 | 3 / 3 | 210 MB | 1.38-1.32 | MISS: p95 35.3 ms > 11 ms (no baseline row) |
| beamer-30 | 58 | split@middle | 19 | 34.6 / 35.5 / 35.6 | p95 | 11 | 35.5 | 117.0 | 0.2 | 19/19 | 3 / 3 | 210 MB | 1.32-1.37 | MISS: p95 35.5 ms > 11 ms (no baseline row) |
| beamer-30 | 58 | letter@start | 19 | 35.8 / 37.1 / 38.7 | p95 | 11 | 37.1 | 86.6 | 0.2 | 19/19 | 2 / 2 | 210 MB | 1.37-1.32 | MISS: p95 37.1 ms > 11 ms (no baseline row) |
| beamer-30 | 58 | letter@middle | 19 | 34.3 / 35.3 / 36.2 | p95 | 11 | 35.3 | 82.6 | 0.2 | 19/19 | 2 / 2 | 210 MB | 1.32-1.29 | MISS: p95 35.3 ms > 11 ms (no baseline row) |
| beamer-30 | 58 | letter@end | 19 | 34.6 / 35.4 / 36.0 | p95 | 11 | 35.4 | 83.6 | 0.1 | 19/19 | 2 / 2 | 210 MB | 1.29-1.4 | MISS: p95 35.4 ms > 11 ms (no baseline row) |
| beamer-30 | 58 | typing@50ms | 39 | 41.2 / 48.2 / 48.3 | p95 | 11 | - | - | 0.4 | - | - | 210 MB | 1.4-1.4 | MISS: p95 48.2 ms > 11 ms |
| beamer-30 | 58 | typing@60ms | 39 | 35.3 / 37.6 / 38.2 | p95 | 11 | - | - | 0.4 | - | - | 210 MB | 1.4-1.45 | MISS: p95 37.6 ms > 11 ms |
| beamer-30 | 58 | typing@80ms | 39 | 35.6 / 37.3 / 38.8 | p95 | 11 | - | - | 0.9 | - | - | 210 MB | 1.45-1.49 | MISS: p95 37.3 ms > 11 ms |
| beamer-30 | 58 | typing@100ms | 39 | 34.1 / 34.5 / 36.5 | p95 | 11 | - | - | 0.1 | - | - | 210 MB | 1.49-1.49 | MISS: p95 34.5 ms > 11 ms |
| beamer-30 | 58 | typing@150ms | 39 | 34.4 / 35.8 / 36.5 | p95 | 11 | - | - | 0.2 | - | - | 210 MB | 1.49-1.41 | MISS: p95 35.8 ms > 11 ms |
| beamer-30 | 58 | preamble | 13 | 408.3 / 415.4 / 419.6 | p95 | 400 | 415.4 | 1719.1 | 1.8 | 0/13 | 58 / 58 | 210 MB | 1.41-1.61 | MISS: p95 415.4 ms > 400 ms (no baseline row) |
| beamer-30 | 58 | reopen: host's share, cold (spawn -> page 1) | 12 | 161.0 / 162.7 / 165.9 | p95 | (100, app) | - | - | - | - | - | 210 MB | 1.45-1.64 | report-only: p95 162.7 ms > 100 ms |
| beamer-30 | 58 | reopen: host already listening (not counted, decision 8) | 12 | 91.6 / 93.7 / 95.7 | p95 | (100, app) | - | - | - | - | - | 210 MB | 1.45-1.64 | report-only |

ms: COMPILE written to the watched page's PAGE frame read on the socket (preamble, reopen: page 1); typing@: the keystroke sent to the watched page from its own compile or a later one. off-CPU: of the host's time to that page, what its engine thread spent descheduled (wall minus queue minus thread CPU): load or a CPU quota, not the engine. The first sample of each phase is a warm-up, left out; rows with n < 12 are gated on their maximum. No noise margin: a row passes only when it meets its target.
