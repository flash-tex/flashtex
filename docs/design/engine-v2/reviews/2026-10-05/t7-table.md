**NON-REFERENCE run**: power source unknown at start; power source unknown at end

| doc | pages | edit | n | ms p50 / p95 / max | gated on | target | first changed page p95 | DONE p95 ms | converged | re-typeset pages p50 / max | host RSS peak | load1 | verdict |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| plain-10 | 10 | letter@start | 19 | 8.2 / 9.0 / 9.0 | p95 | 11 | 9.0 | 33.5 | 19/19 | 2 / 2 | 158 MB | 1.26-1.14 | PASS |
| plain-10 | 10 | letter@middle | 19 | 6.6 / 7.3 / 7.3 | p95 | 11 | 4.1 | 28.4 | 19/19 | 3 / 3 | 158 MB | 1.14-1.13 | PASS |
| plain-10 | 10 | letter@end | 19 | 4.4 / 4.7 / 4.7 | p95 | 11 | 4.7 | 15.5 | 0/19 | 1 / 1 | 158 MB | 1.13-1.04 | PASS |
| plain-10 | 10 | sentence@middle | 19 | 7.5 / 7.9 / 8.4 | p95 | 11 | 4.5 | 33.4 | 0/19 | 6 / 6 | 158 MB | 1.04-1.04 | PASS |
| plain-10 | 10 | newline@middle | 19 | 7.1 / 7.8 / 8.1 | p95 | 11 | 4.5 | 29.4 | 0/19 | 6 / 6 | 158 MB | 1.04-0.96 | PASS |
| plain-10 | 10 | split@middle | 19 | 7.1 / 7.7 / 8.5 | p95 | 11 | 4.4 | 31.9 | 0/19 | 6 / 6 | 158 MB | 0.96-0.88 | PASS |
| plain-10 | 10 | preamble | 13 | 71.1 / 73.5 / 74.7 | p95 | 400 | 73.5 | 104.4 | 0/13 | 10 / 10 | 158 MB | 0.88-0.81 | PASS |
| plain-10 | 10 | reopen: host's share, cold (spawn -> page 1) | 12 | 298.1 / 305.7 / 306.6 | p95 | (100, app) | - | - | - | - | 158 MB | 0.91-1.07 | report-only: p95 305.7 ms > 100 ms |
| plain-10 | 10 | reopen: host already listening (not counted, decision 8) | 12 | 22.2 / 22.8 / 23.0 | p95 | (100, app) | - | - | - | - | 158 MB | 0.91-1.07 | report-only |
| full-10 | 11 | letter@middle | 19 | 11.9 / 13.2 / 13.8 | p95 | 11 | 6.0 | 49.3 | 19/19 | 3 / 3 | 201 MB | 1.07-0.99 | MISS: p95 13.2 ms > 11 ms |
| full-10 | 11 | letter@end | 19 | 15.7 / 17.2 / 18.0 | p95 | 11 | 10.0 | 46.0 | 19/19 | 3 / 3 | 201 MB | 0.99-1.5 | MISS: p95 17.2 ms > 11 ms |
| full-10 | 11 | sentence@middle | 19 | 12.0 / 13.0 / 13.0 | p95 | 11 | 6.0 | 59.4 | 0/19 | 7 / 7 | 201 MB | 1.5-1.38 | MISS: p95 13.0 ms > 11 ms |
| full-10 | 11 | newline@middle | 19 | 13.2 / 13.8 / 13.8 | p95 | 11 | 6.2 | 59.2 | 0/19 | 7 / 7 | 201 MB | 1.38-1.25 | MISS: p95 13.8 ms > 11 ms |
| full-10 | 11 | split@middle | 19 | 12.7 / 13.7 / 19.7 | p95 | 11 | 6.2 | 66.0 | 0/19 | 7 / 7 | 201 MB | 1.25-1.31 | MISS: p95 13.7 ms > 11 ms |
| full-10 | 11 | letter@start | 19 | 17.6 / 19.7 / 20.7 | p95 | 11 | 19.7 | 59.0 | 19/19 | 2 / 2 | 201 MB | 1.31-1.2 | MISS: p95 19.7 ms > 11 ms |
| full-10 | 11 | preamble | 13 | 286.9 / 296.1 / 298.2 | p95 | 400 | 296.1 | 368.0 | 0/13 | 11 / 11 | 201 MB | 1.2-1.65 | PASS |
| full-10 | 11 | reopen: host's share, cold (spawn -> page 1) | 12 | 346.7 / 349.7 / 354.3 | p95 | (100, app) | - | - | - | - | 201 MB | 1.43-1.6 | report-only: p95 349.7 ms > 100 ms |
| full-10 | 11 | reopen: host already listening (not counted, decision 8) | 12 | 75.1 / 77.5 / 77.9 | p95 | (100, app) | - | - | - | - | 201 MB | 1.43-1.6 | report-only |
| plain-100 | 100 | letter@end | 19 | 7.2 / 8.1 / 8.3 | p95 | 11 | 4.7 | 29.0 | 19/19 | 3 / 3 | 212 MB | 1.43-1.47 | PASS |
| plain-100 | 100 | sentence@middle | 19 | 5.3 / 5.9 / 6.2 | p95 | 11 | 5.9 | 170.1 | 0/19 | 50 / 50 | 212 MB | 1.47-1.47 | PASS |
| plain-100 | 100 | newline@middle | 19 | 5.6 / 6.0 / 6.3 | p95 | 11 | 6.0 | 176.9 | 0/19 | 50 / 50 | 212 MB | 1.47-1.32 | PASS |
| plain-100 | 100 | split@middle | 19 | 5.6 / 6.5 / 6.5 | p95 | 11 | 6.5 | 186.3 | 0/19 | 50 / 50 | 212 MB | 1.32-1.2 | PASS |
| plain-100 | 100 | letter@start | 19 | 4.8 / 5.3 / 5.3 | p95 | 11 | 5.3 | 33.9 | 19/19 | 2 / 2 | 212 MB | 1.2-1.1 | PASS |
| plain-100 | 100 | letter@middle | 19 | 5.4 / 5.9 / 5.9 | p95 | 11 | 5.9 | 32.4 | 19/19 | 2 / 2 | 212 MB | 1.1-0.93 | PASS |
| plain-100 | 100 | preamble | 13 | 71.4 / 73.4 / 75.6 | p95 | 400 | 73.4 | 382.5 | 0/13 | 100 / 100 | 212 MB | 0.93-0.86 | PASS |
| plain-100 | 100 | reopen: host's share, cold (spawn -> page 1) | 12 | 295.0 / 300.4 / 300.4 | p95 | (100, app) | - | - | - | - | 212 MB | 0.74-0.86 | report-only: p95 300.4 ms > 100 ms |
| plain-100 | 100 | reopen: host already listening (not counted, decision 8) | 12 | 21.9 / 23.4 / 23.8 | p95 | (100, app) | - | - | - | - | 212 MB | 0.74-0.86 | report-only |
| full-100 | 101 | sentence@middle | 19 | 9.7 / 10.1 / 13.4 | p95 | 11 | 10.1 | 49.6 | 19/19 | 2 / 2 | 285 MB | 0.74-0.78 | PASS |
| full-100 | 101 | newline@middle | 19 | 9.9 / 10.8 / 11.6 | p95 | 11 | 10.8 | 413.4 | 0/19 | 51 / 51 | 285 MB | 0.78-0.89 | PASS |
| full-100 | 101 | split@middle | 19 | 9.7 / 10.4 / 10.5 | p95 | 11 | 10.4 | 407.2 | 0/19 | 51 / 51 | 285 MB | 0.89-1.12 | PASS |
| full-100 | 101 | letter@start | 19 | 10.2 / 11.3 / 11.3 | p95 | 11 | 11.3 | 62.5 | 19/19 | 2 / 2 | 285 MB | 1.12-1.1 | MISS: p95 11.3 ms > 11 ms |
| full-100 | 101 | letter@middle | 19 | 10.2 / 11.4 / 11.8 | p95 | 11 | 11.4 | 60.9 | 19/19 | 2 / 2 | 285 MB | 1.1-1.01 | MISS: p95 11.4 ms > 11 ms |
| full-100 | 101 | letter@end | 19 | 15.0 / 19.2 / 21.6 | p95 | 11 | 9.8 | 62.2 | 19/19 | 3 / 3 | 285 MB | 1.01-1.01 | MISS: p95 19.2 ms > 11 ms |
| full-100 | 101 | preamble | 13 | 299.1 / 314.9 / 322.9 | p95 | 400 | 314.9 | 1071.9 | 0/13 | 101 / 101 | 285 MB | 1.01-1.01 | PASS |
| full-100 | 101 | reopen: host's share, cold (spawn -> page 1) | 12 | 358.5 / 368.9 / 369.3 | p95 | (100, app) | - | - | - | - | 285 MB | 0.93-1.17 | report-only: p95 368.9 ms > 100 ms |
| full-100 | 101 | reopen: host already listening (not counted, decision 8) | 12 | 77.8 / 79.7 / 82.4 | p95 | (100, app) | - | - | - | - | 285 MB | 0.93-1.17 | report-only |
| plain-300 | 301 | newline@middle | 19 | 6.7 / 7.2 / 7.3 | p95 | 11 | 7.2 | 528.8 | 0/19 | 151 / 151 | 374 MB | 1.17-1.43 | PASS |
| plain-300 | 301 | split@middle | 19 | 6.9 / 8.6 / 10.2 | p95 | 11 | 8.6 | 533.0 | 0/19 | 151 / 151 | 374 MB | 1.43-1.63 | PASS |
| plain-300 | 301 | letter@start | 19 | 6.2 / 8.3 / 8.3 | p95 | 11 | 8.3 | 54.1 | 19/19 | 2 / 2 | 374 MB | 1.63-1.5 | PASS |
| plain-300 | 301 | letter@middle | 19 | 6.2 / 6.8 / 7.3 | p95 | 11 | 6.8 | 41.6 | 19/19 | 2 / 2 | 374 MB | 1.5-1.54 | PASS |
| plain-300 | 301 | letter@end | 19 | 6.0 / 6.4 / 6.4 | p95 | 11 | 6.4 | 30.1 | 19/19 | 2 / 2 | 374 MB | 1.54-1.3 | PASS |
| plain-300 | 301 | sentence@middle | 19 | 6.2 / 6.7 / 7.1 | p95 | 11 | 6.7 | 261.7 | 19/19 | 65 / 65 | 374 MB | 1.3-1.18 | PASS |
| plain-300 | 301 | preamble | 13 | 76.4 / 79.0 / 82.8 | p95 | 400 | 79.0 | 1026.2 | 0/13 | 301 / 301 | 374 MB | 1.18-1.29 | PASS |
| plain-300 | 301 | reopen: host's share, cold (spawn -> page 1) | 12 | 312.8 / 323.0 / 324.2 | p95 | (100, app) | - | - | - | - | 374 MB | 1.26-1.73 | report-only: p95 323.0 ms > 100 ms |
| plain-300 | 301 | reopen: host already listening (not counted, decision 8) | 12 | 24.2 / 25.1 / 26.7 | p95 | (100, app) | - | - | - | - | 374 MB | 1.26-1.73 | report-only |
| full-300 | 299 | split@middle | 19 | 13.8 / 15.0 / 16.4 | p95 | 11 | 15.0 | 1223.9 | 0/19 | 150 / 150 | 640 MB | 1.67-1.68 | MISS: p95 15.0 ms > 11 ms |
| full-300 | 299 | letter@start | 19 | 11.7 / 13.3 / 14.0 | p95 | 11 | 13.3 | 101.9 | 19/19 | 2 / 2 | 640 MB | 1.68-1.67 | MISS: p95 13.3 ms > 11 ms |
| full-300 | 299 | letter@middle | 19 | 13.7 / 14.8 / 15.6 | p95 | 11 | 14.8 | 73.5 | 19/19 | 2 / 2 | 640 MB | 1.67-1.61 | MISS: p95 14.8 ms > 11 ms |
| full-300 | 299 | letter@end | 19 | 15.0 / 16.2 / 16.6 | p95 | 11 | 8.2 | 57.7 | 19/19 | 3 / 3 | 640 MB | 1.61-1.44 | MISS: p95 16.2 ms > 11 ms |
| full-300 | 299 | sentence@middle | 19 | 13.9 / 14.6 / 15.0 | p95 | 11 | 14.6 | 347.3 | 19/19 | 33 / 33 | 640 MB | 1.44-1.37 | MISS: p95 14.6 ms > 11 ms |
| full-300 | 299 | newline@middle | 19 | 14.5 / 15.8 / 18.8 | p95 | 11 | 15.8 | 1195.8 | 0/19 | 150 / 150 | 640 MB | 1.37-1.41 | MISS: p95 15.8 ms > 11 ms |
| full-300 | 299 | preamble | 13 | 299.9 / 321.5 / 330.5 | p95 | 400 | 321.5 | 2585.9 | 0/13 | 299 / 299 | 640 MB | 1.41-1.94 | PASS |
| full-300 | 299 | reopen: host's share, cold (spawn -> page 1) | 12 | 355.2 / 373.1 / 376.0 | p95 | (100, app) | - | - | - | - | 640 MB | 1.46-1.86 | report-only: p95 373.1 ms > 100 ms |
| full-300 | 299 | reopen: host already listening (not counted, decision 8) | 12 | 83.9 / 87.6 / 87.6 | p95 | (100, app) | - | - | - | - | 640 MB | 1.46-1.86 | report-only |
| plain-1000 | 1001 | letter@start | 19 | 10.7 / 11.2 / 11.2 | p95 | 11 | 11.2 | 105.0 | 19/19 | 2 / 2 | 1180 MB | 1.5-1.53 | MISS: p95 11.2 ms > 11 ms |
| plain-1000 | 1001 | letter@middle | 19 | 12.1 / 13.2 / 15.0 | p95 | 11 | 9.8 | 76.8 | 19/19 | 3 / 3 | 1180 MB | 1.53-1.6 | MISS: p95 13.2 ms > 11 ms |
| plain-1000 | 1001 | letter@end | 19 | 8.9 / 9.3 / 9.3 | p95 | 11 | 9.3 | 35.3 | 19/19 | 2 / 2 | 1180 MB | 1.6-1.72 | PASS |
| plain-1000 | 1001 | sentence@middle | 19 | 16.6 / 17.8 / 18.5 | p95 | 11 | 14.7 | 4703.4 | 0/19 | 1503 / 1505 | 1180 MB | 1.72-2.35 | MISS: p95 17.8 ms > 11 ms |
| plain-1000 | 1001 | newline@middle | 19 | 14.8 / 16.4 / 17.5 | p95 | 11 | 13.0 | 1721.7 | 0/19 | 502 / 502 | 1180 MB | 2.35-2.84 | MISS: p95 16.4 ms > 11 ms |
| plain-1000 | 1001 | split@middle | 19 | 18.0 / 19.8 / 20.2 | p95 | 11 | 16.3 | 4789.2 | 0/19 | 1503 / 1505 | 1180 MB | 2.84-2.77 | MISS: p95 19.8 ms > 11 ms |
| plain-1000 | 1001 | preamble | 13 | 84.3 / 90.2 / 98.9 | p95 | 400 | 90.2 | 3193.3 | 0/13 | 1001 / 1001 | 1180 MB | 2.77-2.43 | PASS |
| plain-1000 | 1001 | reopen: host's share, cold (spawn -> page 1) | 12 | 307.9 / 320.6 / 330.7 | p95 | (100, app) | - | - | - | - | 1180 MB | 2.09-3.4 | report-only: p95 320.6 ms > 100 ms |
| plain-1000 | 1001 | reopen: host already listening (not counted, decision 8) | 12 | 28.4 / 29.0 / 29.1 | p95 | (100, app) | - | - | - | - | 1180 MB | 2.09-3.4 | report-only |
| full-1000 | 1002 | letter@middle | 19 | 17.3 / 19.0 / 20.3 | p95 | 11 | 19.0 | 380.1 | 19/19 | 33 / 33 | 1494 MB | 2.09-1.92 | MISS: p95 19.0 ms > 11 ms |
| full-1000 | 1002 | letter@end | 19 | 16.3 / 18.2 / 18.4 | p95 | 11 | 11.9 | 73.6 | 19/19 | 3 / 3 | 1494 MB | 1.92-1.77 | MISS: p95 18.2 ms > 11 ms |
| full-1000 | 1002 | sentence@middle | 19 | 16.8 / 17.8 / 18.0 | p95 | 11 | 17.8 | 373.6 | 19/19 | 33 / 33 | 1494 MB | 1.77-1.68 | MISS: p95 17.8 ms > 11 ms |
| full-1000 | 1002 | newline@middle | 19 | 19.3 / 20.3 / 21.1 | p95 | 11 | 20.3 | 3972.2 | 0/19 | 501 / 501 | 1494 MB | 1.68-1.61 | MISS: p95 20.3 ms > 11 ms |
| full-1000 | 1002 | split@middle | 19 | 19.8 / 21.6 / 24.1 | p95 | 11 | 21.6 | 3976.9 | 0/19 | 501 / 501 | 1494 MB | 1.61-1.69 | MISS: p95 21.6 ms > 11 ms |
| full-1000 | 1002 | letter@start | 19 | 18.1 / 19.3 / 21.3 | p95 | 11 | 19.3 | 172.2 | 19/19 | 2 / 2 | 1494 MB | 1.69-1.43 | MISS: p95 19.3 ms > 11 ms |
| full-1000 | 1002 | preamble | 13 | 328.9 / 335.0 / 341.2 | p95 | 400 | 335.0 | 7735.1 | 0/13 | 1002 / 1002 | 1494 MB | 1.43-1.95 | PASS |
| full-1000 | 1002 | reopen: host's share, cold (spawn -> page 1) | 12 | 375.0 / 383.7 / 390.1 | p95 | (100, app) | - | - | - | - | 1494 MB | 1.45-1.91 | report-only: p95 383.7 ms > 100 ms |
| full-1000 | 1002 | reopen: host already listening (not counted, decision 8) | 12 | 99.8 / 102.2 / 105.3 | p95 | (100, app) | - | - | - | - | 1494 MB | 1.45-1.91 | report-only: p95 102.2 ms > 100 ms |

ms: COMPILE written to the watched page's PAGE frame read on the socket (preamble, reopen: page 1). The first sample of each phase is a warm-up, left out; rows with n < 12 are gated on their maximum. No noise margin: a row passes only when it meets its target.
