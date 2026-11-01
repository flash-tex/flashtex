# T7 reference run, 2026-10-21, Kabirs-MacBook-Pro

P4 gate (DESIGN.md §12; Q1 decided 2026-10-05): `scripts/t7-reference.sh`, the full T7 run on an
idle, plugged-in Apple-Silicon Mac with Low Power Mode off.

- **Verdict:** T7: PASS (120 of 120 gated rows pass; NON-REFERENCE (on battery at start (48 %); on battery at end (40 %)) run)
- **Exit:** 0: every gated row meets its target, on a reference run
- **Commit:** `58d6d773f` (Merge pull request #1875 from flash-tex/agent/kabir-claude/t7-headroom)
- **Machine:** Mac17,9 Apple M5 Pro 26.6.2
- **Checks failed (--force):** not on mains power: Now drawing from 'Battery Power'
- **Command:** `scripts/t7-reference.sh --max-load 100 --wait 0 --force --out docs/evidence/t7-reference-2026-10-21e-Kabirs-MacBook-Pro`, which ran `t7.py --engine ref --out /tmp/flashtex-t7-reference/t7-reference-run --max-load 100 --wait-load 100 --wait-max 0 --build`

Files: `table.md` (the rows), `summary.json.gz` (`t7.py --check` re-evaluates it after
gunzip), `raw.tgz` (dl3-keys' lines), `environment.txt`, `t7-output.txt`.

**NON-REFERENCE run**: on battery at start (48 %); on battery at end (40 %)

| doc | pages | edit | n | ms p50 / p95 / max | gated on | target | first changed page p95 | DONE p95 ms | off-CPU p95 ms | converged | re-typeset pages p50 / max | host RSS peak | load1 | verdict |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| plain-10 | 10 | letter@start | 19 | 6.6 / 8.8 / 9.1 | p95 | 11 | 8.8 | 23.0 | 2.2 | 19/19 | 2 / 2 | 167 MB | 1.96-2.04 | PASS |
| plain-10 | 10 | letter@middle | 19 | 3.8 / 4.8 / 4.8 | p95 | 11 | 3.1 | 15.8 | 0.1 | 19/19 | 3 / 3 | 167 MB | 2.04-2.04 | PASS |
| plain-10 | 10 | letter@end | 19 | 3.1 / 3.8 / 3.8 | p95 | 11 | 3.8 | 9.2 | 0.2 | 0/19 | 1 / 1 | 167 MB | 2.04-1.88 | PASS |
| plain-10 | 10 | sentence@middle | 19 | 3.7 / 4.5 / 4.7 | p95 | 11 | 2.8 | 19.5 | 0.1 | 0/19 | 6 / 6 | 167 MB | 1.88-2.13 | PASS |
| plain-10 | 10 | newline@middle | 19 | 4.0 / 4.8 / 4.8 | p95 | 11 | 3.2 | 16.0 | 0.1 | 19/19 | 3 / 3 | 167 MB | 2.13-2.04 | PASS |
| plain-10 | 10 | split@middle | 19 | 4.1 / 4.8 / 5.2 | p95 | 11 | 3.1 | 20.4 | 0.1 | 0/19 | 6 / 6 | 167 MB | 2.04-2.12 | PASS |
| plain-10 | 10 | typing@50ms | 39 | 2.9 / 3.3 / 3.5 | p95 | 11 | - | - | 0.1 | - | - | 167 MB | 2.12-2.12 | PASS |
| plain-10 | 10 | typing@60ms | 39 | 2.9 / 3.3 / 3.5 | p95 | 11 | - | - | 0.1 | - | - | 167 MB | 2.12-2.03 | PASS |
| plain-10 | 10 | typing@80ms | 39 | 3.1 / 3.5 / 3.7 | p95 | 11 | - | - | 0.1 | - | - | 167 MB | 2.03-2.03 | PASS |
| plain-10 | 10 | typing@100ms | 39 | 3.3 / 3.6 / 3.7 | p95 | 11 | - | - | 0.1 | - | - | 167 MB | 2.03-1.95 | PASS |
| plain-10 | 10 | typing@150ms | 39 | 3.6 / 4.2 / 4.4 | p95 | 11 | - | - | 0.1 | - | - | 167 MB | 1.95-1.79 | PASS |
| plain-10 | 10 | preamble | 13 | 32.2 / 35.2 / 40.6 | p95 | 400 | 35.2 | 48.5 | 1.2 | 0/13 | 10 / 10 | 167 MB | 1.79-1.73 | PASS |
| plain-10 | 10 | reopen: host's share, cold (spawn -> page 1) | 12 | 59.4 / 62.9 / 65.5 | p95 | (100, app) | - | - | - | - | - | 167 MB | 1.91-1.99 | report-only |
| plain-10 | 10 | reopen: host already listening (not counted, decision 8) | 12 | 18.2 / 19.7 / 20.0 | p95 | (100, app) | - | - | - | - | - | 167 MB | 1.91-1.99 | report-only |
| full-10 | 11 | letter@middle | 19 | 7.0 / 8.6 / 8.6 | p95 | 11 | 3.9 | 29.4 | 0.2 | 19/19 | 3 / 3 | 236 MB | 1.91-2.16 | PASS |
| full-10 | 11 | letter@end | 19 | 7.5 / 8.4 / 10.2 | p95 | 11 | 4.6 | 23.8 | 0.3 | 19/19 | 3 / 3 | 236 MB | 2.16-1.9 | PASS |
| full-10 | 11 | sentence@middle | 19 | 7.8 / 8.3 / 8.4 | p95 | 11 | 4.7 | 34.8 | 0.2 | 0/19 | 7 / 7 | 236 MB | 1.9-1.83 | PASS |
| full-10 | 11 | newline@middle | 19 | 7.6 / 8.2 / 8.5 | p95 | 11 | 4.4 | 30.7 | 0.2 | 19/19 | 3 / 3 | 236 MB | 1.83-1.84 | PASS |
| full-10 | 11 | split@middle | 19 | 7.7 / 11.0 / 27.9 | p95 | 11 | 5.5 | 38.2 | 0.7 | 19/19 | 3 / 3 | 236 MB | 1.84-1.71 | PASS |
| full-10 | 11 | letter@start | 19 | 8.1 / 9.1 / 9.5 | p95 | 11 | 9.1 | 34.1 | 0.7 | 19/19 | 2 / 2 | 236 MB | 1.71-1.89 | PASS |
| full-10 | 11 | typing@50ms | 39 | 6.6 / 8.0 / 8.5 | p95 | 11 | - | - | 0.2 | - | - | 236 MB | 1.89-1.89 | PASS |
| full-10 | 11 | typing@60ms | 39 | 6.8 / 7.6 / 8.1 | p95 | 11 | - | - | 0.2 | - | - | 236 MB | 1.89-2.06 | PASS |
| full-10 | 11 | typing@80ms | 39 | 6.8 / 7.5 / 7.9 | p95 | 11 | - | - | 0.2 | - | - | 236 MB | 2.06-2.05 | PASS |
| full-10 | 11 | typing@100ms | 39 | 6.8 / 7.9 / 9.4 | p95 | 11 | - | - | 0.2 | - | - | 236 MB | 2.05-2.05 | PASS |
| full-10 | 11 | typing@150ms | 39 | 6.9 / 7.8 / 8.1 | p95 | 11 | - | - | 0.2 | - | - | 236 MB | 2.05-1.82 | PASS |
| full-10 | 11 | preamble | 13 | 158.3 / 162.6 / 168.1 | p95 | 400 | 162.6 | 203.5 | 2.5 | 0/13 | 11 / 11 | 236 MB | 1.82-1.75 | PASS |
| full-10 | 11 | reopen: host's share, cold (spawn -> page 1) | 12 | 90.9 / 101.9 / 119.8 | p95 | (100, app) | - | - | - | - | - | 236 MB | 1.48-1.75 | report-only: p95 101.9 ms > 100 ms |
| full-10 | 11 | reopen: host already listening (not counted, decision 8) | 12 | 51.7 / 57.7 / 69.9 | p95 | (100, app) | - | - | - | - | - | 236 MB | 1.48-1.75 | report-only |
| plain-100 | 100 | letter@end | 19 | 4.3 / 7.4 / 8.4 | p95 | 11 | 5.8 | 18.7 | 2.5 | 19/19 | 3 / 3 | 195 MB | 1.48-1.44 | PASS |
| plain-100 | 100 | sentence@middle | 19 | 3.0 / 4.0 / 4.3 | p95 | 11 | 4.0 | 81.5 | 0.2 | 0/19 | 50 / 50 | 195 MB | 1.44-1.37 | PASS |
| plain-100 | 100 | newline@middle | 19 | 3.0 / 4.1 / 4.1 | p95 | 11 | 4.1 | 14.3 | 0.1 | 19/19 | 2 / 2 | 195 MB | 1.37-1.34 | PASS |
| plain-100 | 100 | split@middle | 19 | 3.0 / 4.2 / 4.3 | p95 | 11 | 4.2 | 17.1 | 0.1 | 19/19 | 2 / 2 | 195 MB | 1.34-1.39 | PASS |
| plain-100 | 100 | letter@start | 19 | 3.6 / 4.5 / 4.5 | p95 | 11 | 4.5 | 18.1 | 0.4 | 19/19 | 2 / 2 | 195 MB | 1.39-1.25 | PASS |
| plain-100 | 100 | letter@middle | 19 | 2.9 / 4.2 / 4.2 | p95 | 11 | 4.2 | 16.9 | 0.1 | 19/19 | 2 / 2 | 195 MB | 1.25-1.15 | PASS |
| plain-100 | 100 | typing@50ms | 39 | 2.5 / 2.8 / 3.1 | p95 | 11 | - | - | 0.1 | - | - | 195 MB | 1.15-1.15 | PASS |
| plain-100 | 100 | typing@60ms | 39 | 2.6 / 2.9 / 2.9 | p95 | 11 | - | - | 0.1 | - | - | 195 MB | 1.15-1.06 | PASS |
| plain-100 | 100 | typing@80ms | 39 | 2.7 / 3.0 / 3.0 | p95 | 11 | - | - | 0.1 | - | - | 195 MB | 1.06-1.06 | PASS |
| plain-100 | 100 | typing@100ms | 39 | 2.7 / 3.0 / 3.1 | p95 | 11 | - | - | 0.1 | - | - | 195 MB | 1.06-1.05 | PASS |
| plain-100 | 100 | typing@150ms | 39 | 2.9 / 3.4 / 3.7 | p95 | 11 | - | - | 0.1 | - | - | 195 MB | 1.05-1.13 | PASS |
| plain-100 | 100 | preamble | 13 | 33.3 / 34.6 / 43.8 | p95 | 400 | 34.6 | 159.4 | 1.2 | 0/13 | 100 / 100 | 195 MB | 1.13-1.99 | PASS |
| plain-100 | 100 | reopen: host's share, cold (spawn -> page 1) | 12 | 55.4 / 56.6 / 60.2 | p95 | (100, app) | - | - | - | - | - | 195 MB | 1.69-1.99 | report-only |
| plain-100 | 100 | reopen: host already listening (not counted, decision 8) | 12 | 16.5 / 16.9 / 18.6 | p95 | (100, app) | - | - | - | - | - | 195 MB | 1.69-1.99 | report-only |
| full-100 | 101 | sentence@middle | 19 | 5.5 / 6.4 / 6.6 | p95 | 11 | 6.4 | 27.2 | 0.2 | 19/19 | 2 / 2 | 288 MB | 1.69-1.55 | PASS |
| full-100 | 101 | newline@middle | 19 | 5.5 / 6.7 / 7.0 | p95 | 11 | 6.7 | 26.0 | 0.2 | 19/19 | 2 / 2 | 288 MB | 1.55-1.39 | PASS |
| full-100 | 101 | split@middle | 19 | 5.6 / 6.2 / 6.3 | p95 | 11 | 6.2 | 31.7 | 0.2 | 19/19 | 2 / 2 | 288 MB | 1.39-1.36 | PASS |
| full-100 | 101 | letter@start | 19 | 5.7 / 7.3 / 7.4 | p95 | 11 | 7.3 | 27.4 | 0.3 | 19/19 | 2 / 2 | 288 MB | 1.36-1.33 | PASS |
| full-100 | 101 | letter@middle | 19 | 5.5 / 6.4 / 7.1 | p95 | 11 | 6.4 | 27.0 | 0.2 | 19/19 | 2 / 2 | 288 MB | 1.33-1.42 | PASS |
| full-100 | 101 | letter@end | 19 | 7.4 / 8.6 / 9.2 | p95 | 11 | 4.3 | 30.4 | 0.2 | 19/19 | 3 / 3 | 288 MB | 1.42-1.71 | PASS |
| full-100 | 101 | typing@50ms | 39 | 5.0 / 5.7 / 5.8 | p95 | 11 | - | - | 0.2 | - | - | 288 MB | 1.71-1.71 | PASS |
| full-100 | 101 | typing@60ms | 39 | 5.2 / 6.8 / 9.0 | p95 | 11 | - | - | 0.4 | - | - | 288 MB | 1.71-1.57 | PASS |
| full-100 | 101 | typing@80ms | 39 | 5.3 / 6.4 / 6.9 | p95 | 11 | - | - | 0.3 | - | - | 288 MB | 1.57-1.6 | PASS |
| full-100 | 101 | typing@100ms | 39 | 5.2 / 5.7 / 7.2 | p95 | 11 | - | - | 0.2 | - | - | 288 MB | 1.6-1.6 | PASS |
| full-100 | 101 | typing@150ms | 39 | 5.3 / 5.9 / 8.0 | p95 | 11 | - | - | 0.2 | - | - | 288 MB | 1.6-1.75 | PASS |
| full-100 | 101 | preamble | 13 | 167.2 / 170.3 / 186.0 | p95 | 400 | 170.3 | 607.6 | 2.6 | 0/13 | 101 / 101 | 288 MB | 1.75-2.03 | PASS |
| full-100 | 101 | reopen: host's share, cold (spawn -> page 1) | 12 | 92.5 / 94.2 / 94.6 | p95 | (100, app) | - | - | - | - | - | 288 MB | 1.88-1.96 | report-only |
| full-100 | 101 | reopen: host already listening (not counted, decision 8) | 12 | 52.2 / 54.1 / 55.6 | p95 | (100, app) | - | - | - | - | - | 288 MB | 1.88-1.96 | report-only |
| plain-300 | 301 | newline@middle | 19 | 3.1 / 3.3 / 7.2 | p95 | 11 | 3.3 | 26.7 | 0.1 | 19/19 | 4 / 4 | 289 MB | 1.88-1.74 | PASS |
| plain-300 | 301 | split@middle | 19 | 3.1 / 3.2 / 3.2 | p95 | 11 | 3.2 | 40.5 | 0.1 | 19/19 | 9 / 9 | 289 MB | 1.74-2.0 | PASS |
| plain-300 | 301 | letter@start | 19 | 2.9 / 3.4 / 4.0 | p95 | 11 | 3.4 | 26.1 | 0.1 | 19/19 | 2 / 2 | 289 MB | 2.0-1.84 | PASS |
| plain-300 | 301 | letter@middle | 19 | 2.9 / 4.7 / 4.9 | p95 | 11 | 4.7 | 19.1 | 0.1 | 19/19 | 2 / 2 | 289 MB | 1.84-1.72 | PASS |
| plain-300 | 301 | letter@end | 19 | 3.0 / 4.0 / 5.2 | p95 | 11 | 4.0 | 28.5 | 0.3 | 19/19 | 2 / 2 | 289 MB | 1.72-2.22 | PASS |
| plain-300 | 301 | sentence@middle | 19 | 3.1 / 4.0 / 4.6 | p95 | 11 | 4.0 | 121.5 | 0.1 | 19/19 | 65 / 65 | 289 MB | 2.22-2.04 | PASS |
| plain-300 | 301 | typing@50ms | 39 | 2.7 / 2.9 / 3.0 | p95 | 11 | - | - | 0.1 | - | - | 289 MB | 2.04-2.04 | PASS |
| plain-300 | 301 | typing@60ms | 39 | 2.6 / 2.8 / 2.8 | p95 | 11 | - | - | 0.1 | - | - | 289 MB | 2.04-2.04 | PASS |
| plain-300 | 301 | typing@80ms | 39 | 2.6 / 3.0 / 3.6 | p95 | 11 | - | - | 0.1 | - | - | 289 MB | 2.04-2.04 | PASS |
| plain-300 | 301 | typing@100ms | 39 | 2.7 / 3.1 / 3.7 | p95 | 11 | - | - | 0.2 | - | - | 289 MB | 2.04-1.95 | PASS |
| plain-300 | 301 | typing@150ms | 39 | 2.8 / 3.3 / 3.6 | p95 | 11 | - | - | 0.1 | - | - | 289 MB | 1.95-1.88 | PASS |
| plain-300 | 301 | preamble | 13 | 34.5 / 35.4 / 35.4 | p95 | 400 | 35.4 | 414.6 | 1.0 | 0/13 | 301 / 301 | 289 MB | 1.88-1.97 | PASS |
| plain-300 | 301 | reopen: host's share, cold (spawn -> page 1) | 12 | 56.0 / 59.5 / 61.9 | p95 | (100, app) | - | - | - | - | - | 289 MB | 1.89-1.98 | report-only |
| plain-300 | 301 | reopen: host already listening (not counted, decision 8) | 12 | 17.0 / 18.1 / 19.6 | p95 | (100, app) | - | - | - | - | - | 289 MB | 1.89-1.98 | report-only |
| full-300 | 299 | split@middle | 19 | 6.6 / 7.8 / 8.1 | p95 | 11 | 7.8 | 178.7 | 0.2 | 19/19 | 33 / 33 | 447 MB | 1.98-2.13 | PASS |
| full-300 | 299 | letter@start | 19 | 5.6 / 6.0 / 6.3 | p95 | 11 | 6.0 | 33.8 | 0.2 | 19/19 | 2 / 2 | 447 MB | 2.13-2.12 | PASS |
| full-300 | 299 | letter@middle | 19 | 6.9 / 8.4 / 9.3 | p95 | 11 | 8.4 | 33.2 | 0.2 | 19/19 | 2 / 2 | 447 MB | 2.12-2.02 | PASS |
| full-300 | 299 | letter@end | 19 | 7.4 / 8.7 / 9.2 | p95 | 11 | 4.7 | 29.9 | 0.1 | 19/19 | 3 / 3 | 447 MB | 2.02-1.94 | PASS |
| full-300 | 299 | sentence@middle | 19 | 7.0 / 8.4 / 8.6 | p95 | 11 | 8.4 | 179.5 | 0.2 | 19/19 | 33 / 33 | 447 MB | 1.94-1.94 | PASS |
| full-300 | 299 | newline@middle | 19 | 6.8 / 7.8 / 7.8 | p95 | 11 | 7.8 | 30.2 | 0.2 | 19/19 | 2 / 2 | 447 MB | 1.94-2.59 | PASS |
| full-300 | 299 | typing@50ms | 39 | 6.1 / 6.9 / 7.1 | p95 | 11 | - | - | 0.2 | - | - | 447 MB | 2.59-2.62 | PASS |
| full-300 | 299 | typing@60ms | 39 | 6.0 / 6.3 / 6.6 | p95 | 11 | - | - | 0.2 | - | - | 447 MB | 2.62-2.62 | PASS |
| full-300 | 299 | typing@80ms | 39 | 6.3 / 6.8 / 8.7 | p95 | 11 | - | - | 0.2 | - | - | 447 MB | 2.62-2.65 | PASS |
| full-300 | 299 | typing@100ms | 39 | 6.4 / 8.1 / 9.0 | p95 | 11 | - | - | 0.3 | - | - | 447 MB | 2.65-2.65 | PASS |
| full-300 | 299 | typing@150ms | 39 | 6.6 / 7.2 / 7.3 | p95 | 11 | - | - | 0.2 | - | - | 447 MB | 2.65-2.68 | PASS |
| full-300 | 299 | preamble | 13 | 170.3 / 177.9 / 178.1 | p95 | 400 | 177.9 | 1522.1 | 2.6 | 0/13 | 299 / 299 | 447 MB | 2.68-2.85 | PASS |
| full-300 | 299 | reopen: host's share, cold (spawn -> page 1) | 12 | 94.1 / 95.3 / 106.4 | p95 | (100, app) | - | - | - | - | - | 447 MB | 2.59-2.87 | report-only |
| full-300 | 299 | reopen: host already listening (not counted, decision 8) | 12 | 55.0 / 55.7 / 60.9 | p95 | (100, app) | - | - | - | - | - | 447 MB | 2.59-2.87 | report-only |
| plain-1000 | 1001 | letter@start | 19 | 3.2 / 3.3 / 3.4 | p95 | 11 | 3.3 | 29.3 | 0.1 | 19/19 | 2 / 2 | 577 MB | 2.49-2.26 | PASS |
| plain-1000 | 1001 | letter@middle | 19 | 4.1 / 5.5 / 5.9 | p95 | 11 | 4.1 | 24.3 | 0.1 | 19/19 | 3 / 3 | 577 MB | 2.26-2.24 | PASS |
| plain-1000 | 1001 | letter@end | 19 | 3.1 / 4.5 / 4.5 | p95 | 11 | 4.5 | 17.9 | 0.1 | 19/19 | 2 / 2 | 577 MB | 2.24-2.3 | PASS |
| plain-1000 | 1001 | sentence@middle | 19 | 5.2 / 6.4 / 6.7 | p95 | 11 | 5.0 | 2916.2 | 0.2 | 0/19 | 1503 / 1505 | 577 MB | 2.3-4.04 | PASS |
| plain-1000 | 1001 | newline@middle | 19 | 4.6 / 5.6 / 5.7 | p95 | 11 | 4.0 | 25.3 | 0.1 | 19/19 | 3 / 3 | 577 MB | 4.04-3.71 | PASS |
| plain-1000 | 1001 | split@middle | 19 | 5.3 / 6.3 / 6.4 | p95 | 11 | 4.8 | 2904.1 | 0.1 | 0/19 | 1503 / 1505 | 577 MB | 3.71-2.64 | PASS |
| plain-1000 | 1001 | typing@50ms | 39 | 4.1 / 4.8 / 6.5 | p95 | 11 | - | - | 0.2 | - | - | 577 MB | 2.64-2.51 | PASS |
| plain-1000 | 1001 | typing@60ms | 39 | 4.1 / 4.6 / 4.7 | p95 | 11 | - | - | 0.1 | - | - | 577 MB | 2.51-2.51 | PASS |
| plain-1000 | 1001 | typing@80ms | 39 | 4.0 / 4.5 / 4.5 | p95 | 11 | - | - | 0.1 | - | - | 577 MB | 2.51-2.39 | PASS |
| plain-1000 | 1001 | typing@100ms | 39 | 4.1 / 4.5 / 5.8 | p95 | 11 | - | - | 0.1 | - | - | 577 MB | 2.39-2.28 | PASS |
| plain-1000 | 1001 | typing@150ms | 39 | 4.3 / 5.1 / 6.7 | p95 | 11 | - | - | 0.2 | - | - | 577 MB | 2.28-2.09 | PASS |
| plain-1000 | 1001 | preamble | 13 | 41.7 / 43.6 / 47.9 | p95 | 400 | 43.6 | 1381.9 | 1.1 | 0/13 | 1001 / 1001 | 577 MB | 2.09-2.5 | PASS |
| plain-1000 | 1001 | reopen: host's share, cold (spawn -> page 1) | 12 | 58.0 / 59.6 / 61.4 | p95 | (100, app) | - | - | - | - | - | 577 MB | 1.97-2.38 | report-only |
| plain-1000 | 1001 | reopen: host already listening (not counted, decision 8) | 12 | 19.2 / 19.6 / 20.0 | p95 | (100, app) | - | - | - | - | - | 577 MB | 1.97-2.38 | report-only |
| full-1000 | 1002 | letter@middle | 19 | 7.1 / 8.5 / 9.0 | p95 | 11 | 8.5 | 203.6 | 0.2 | 19/19 | 33 / 33 | 835 MB | 1.97-1.71 | PASS |
| full-1000 | 1002 | letter@end | 19 | 6.8 / 8.7 / 8.9 | p95 | 11 | 5.1 | 45.1 | 0.2 | 19/19 | 3 / 3 | 835 MB | 1.71-1.9 | PASS |
| full-1000 | 1002 | sentence@middle | 19 | 7.6 / 8.3 / 8.5 | p95 | 11 | 8.3 | 198.5 | 0.2 | 19/19 | 33 / 33 | 835 MB | 1.9-1.83 | PASS |
| full-1000 | 1002 | newline@middle | 19 | 7.3 / 8.4 / 8.5 | p95 | 11 | 8.4 | 47.0 | 0.2 | 19/19 | 3 / 3 | 835 MB | 1.83-2.0 | PASS |
| full-1000 | 1002 | split@middle | 19 | 7.7 / 8.8 / 8.9 | p95 | 11 | 8.8 | 198.8 | 0.2 | 19/19 | 33 / 33 | 835 MB | 2.0-1.77 | PASS |
| full-1000 | 1002 | letter@start | 19 | 6.9 / 8.6 / 8.9 | p95 | 11 | 8.6 | 49.1 | 0.2 | 19/19 | 2 / 2 | 835 MB | 1.77-1.62 | PASS |
| full-1000 | 1002 | typing@50ms | 39 | 6.7 / 7.4 / 8.4 | p95 | 11 | - | - | 0.3 | - | - | 835 MB | 1.62-1.57 | PASS |
| full-1000 | 1002 | typing@60ms | 39 | 6.8 / 7.2 / 8.1 | p95 | 11 | - | - | 0.4 | - | - | 835 MB | 1.57-1.57 | PASS |
| full-1000 | 1002 | typing@80ms | 39 | 7.1 / 8.0 / 16.0 | p95 | 11 | - | - | 0.4 | - | - | 835 MB | 1.57-1.69 | PASS |
| full-1000 | 1002 | typing@100ms | 39 | 7.0 / 7.6 / 7.8 | p95 | 11 | - | - | 0.4 | - | - | 835 MB | 1.69-1.71 | PASS |
| full-1000 | 1002 | typing@150ms | 39 | 7.5 / 8.1 / 9.1 | p95 | 11 | - | - | 0.4 | - | - | 835 MB | 1.71-1.66 | PASS |
| full-1000 | 1002 | preamble | 13 | 183.6 / 186.0 / 193.6 | p95 | 400 | 186.0 | 5186.6 | 2.4 | 0/13 | 1002 / 1002 | 835 MB | 1.66-2.05 | PASS |
| full-1000 | 1002 | reopen: host's share, cold (spawn -> page 1) | 12 | 101.4 / 105.1 / 107.4 | p95 | (100, app) | - | - | - | - | - | 835 MB | 1.94-2.36 | report-only: p95 105.1 ms > 100 ms |
| full-1000 | 1002 | reopen: host already listening (not counted, decision 8) | 12 | 62.1 / 63.8 / 65.3 | p95 | (100, app) | - | - | - | - | - | 835 MB | 1.94-2.36 | report-only |
| beamer-5 | 5 | letter@end | 19 | 3.9 / 5.6 / 5.7 | p95 | 11 | 5.6 | 26.9 | 0.1 | 19/19 | 2 / 2 | 252 MB | 2.02-1.94 | PASS |
| beamer-5 | 5 | sentence@middle | 19 | 4.0 / 5.6 / 6.0 | p95 | 11 | 5.6 | 19.7 | 0.2 | 19/19 | 2 / 2 | 252 MB | 1.94-2.01 | PASS |
| beamer-5 | 5 | newline@middle | 19 | 4.6 / 5.8 / 5.9 | p95 | 11 | 5.8 | 25.1 | 1.1 | 19/19 | 2 / 3 | 252 MB | 2.01-1.93 | PASS |
| beamer-5 | 5 | split@middle | 19 | 3.9 / 5.1 / 5.4 | p95 | 11 | 5.1 | 44.3 | 0.1 | 19/19 | 3 / 3 | 252 MB | 1.93-1.77 | PASS |
| beamer-5 | 5 | letter@start | 19 | 4.0 / 5.3 / 5.6 | p95 | 11 | 5.3 | 19.3 | 0.2 | 19/19 | 2 / 2 | 252 MB | 1.77-1.58 | PASS |
| beamer-5 | 5 | letter@middle | 19 | 4.0 / 5.6 / 5.7 | p95 | 11 | 5.6 | 20.1 | 0.1 | 19/19 | 2 / 2 | 252 MB | 1.58-1.61 | PASS |
| beamer-5 | 5 | typing@50ms | 39 | 3.5 / 4.7 / 8.4 | p95 | 11 | - | - | 1.1 | - | - | 252 MB | 1.61-1.61 | PASS |
| beamer-5 | 5 | typing@60ms | 39 | 3.6 / 5.0 / 9.5 | p95 | 11 | - | - | 0.3 | - | - | 252 MB | 1.61-2.61 | PASS |
| beamer-5 | 5 | typing@80ms | 39 | 3.6 / 3.9 / 4.1 | p95 | 11 | - | - | 0.1 | - | - | 252 MB | 2.61-2.61 | PASS |
| beamer-5 | 5 | typing@100ms | 39 | 3.6 / 4.0 / 4.2 | p95 | 11 | - | - | 0.1 | - | - | 252 MB | 2.61-2.48 | PASS |
| beamer-5 | 5 | typing@150ms | 39 | 3.9 / 5.1 / 6.3 | p95 | 11 | - | - | 0.2 | - | - | 252 MB | 2.48-2.44 | PASS |
| beamer-5 | 5 | preamble | 13 | 223.3 / 227.9 / 229.8 | p95 | 400 | 227.9 | 270.8 | 5.1 | 0/13 | 5 / 5 | 252 MB | 2.44-2.14 | PASS |
| beamer-5 | 5 | reopen: host's share, cold (spawn -> page 1) | 12 | 89.8 / 93.4 / 95.4 | p95 | (100, app) | - | - | - | - | - | 252 MB | 2.04-2.14 | report-only |
| beamer-5 | 5 | reopen: host already listening (not counted, decision 8) | 12 | 50.4 / 51.6 / 54.9 | p95 | (100, app) | - | - | - | - | - | 252 MB | 2.04-2.14 | report-only |
| beamer-30 | 58 | sentence@middle | 19 | 7.7 / 9.5 / 9.5 | p95 | 11 | 9.5 | 39.4 | 0.1 | 19/19 | 2 / 2 | 309 MB | 2.04-1.87 | PASS |
| beamer-30 | 58 | newline@middle | 19 | 7.5 / 9.3 / 9.4 | p95 | 11 | 9.3 | 37.2 | 0.1 | 19/19 | 3 / 3 | 309 MB | 1.87-1.72 | PASS |
| beamer-30 | 58 | split@middle | 19 | 8.1 / 9.1 / 9.5 | p95 | 11 | 9.1 | 58.6 | 0.1 | 19/19 | 3 / 3 | 309 MB | 1.72-1.62 | PASS |
| beamer-30 | 58 | letter@start | 19 | 8.7 / 10.4 / 10.8 | p95 | 11 | 10.4 | 47.5 | 0.7 | 19/19 | 2 / 2 | 309 MB | 1.62-1.49 | PASS |
| beamer-30 | 58 | letter@middle | 19 | 9.1 / 9.5 / 9.7 | p95 | 11 | 9.5 | 28.5 | 0.1 | 19/19 | 2 / 2 | 309 MB | 1.49-1.37 | PASS |
| beamer-30 | 58 | letter@end | 19 | 7.8 / 9.0 / 9.3 | p95 | 11 | 9.0 | 39.8 | 0.1 | 19/19 | 2 / 2 | 309 MB | 1.37-1.24 | PASS |
| beamer-30 | 58 | typing@50ms | 39 | 6.6 / 7.3 / 7.6 | p95 | 11 | - | - | 0.1 | - | - | 309 MB | 1.24-1.24 | PASS |
| beamer-30 | 58 | typing@60ms | 39 | 6.9 / 7.6 / 8.0 | p95 | 11 | - | - | 0.1 | - | - | 309 MB | 1.24-1.54 | PASS |
| beamer-30 | 58 | typing@80ms | 39 | 7.2 / 7.6 / 8.0 | p95 | 11 | - | - | 0.1 | - | - | 309 MB | 1.54-1.54 | PASS |
| beamer-30 | 58 | typing@100ms | 39 | 7.3 / 7.8 / 13.2 | p95 | 11 | - | - | 0.1 | - | - | 309 MB | 1.54-1.66 | PASS |
| beamer-30 | 58 | typing@150ms | 39 | 7.7 / 8.6 / 9.2 | p95 | 11 | - | - | 0.1 | - | - | 309 MB | 1.66-1.52 | PASS |
| beamer-30 | 58 | preamble | 13 | 243.9 / 248.0 / 261.5 | p95 | 400 | 248.0 | 1061.1 | 5.3 | 0/13 | 58 / 58 | 309 MB | 1.52-1.57 | PASS |
| beamer-30 | 58 | reopen: host's share, cold (spawn -> page 1) | 12 | 107.3 / 107.7 / 118.7 | p95 | (100, app) | - | - | - | - | - | 309 MB | 1.36-1.58 | report-only: p95 107.7 ms > 100 ms |
| beamer-30 | 58 | reopen: host already listening (not counted, decision 8) | 12 | 67.1 / 68.8 / 70.6 | p95 | (100, app) | - | - | - | - | - | 309 MB | 1.36-1.58 | report-only |

ms: COMPILE written to the watched page's PAGE frame read on the socket (preamble, reopen: page 1); typing@: the keystroke sent to the watched page from its own compile or a later one. off-CPU: of the host's time to that page, what its engine thread spent descheduled (wall minus queue minus thread CPU): load or a CPU quota, not the engine. The first sample of each phase is a warm-up, left out; rows with n < 12 are gated on their maximum. No noise margin: a row passes only when it meets its target.

## Owner ruling (2026-10-21)

The machine checks were skipped by the owner's standing direction for this Mac ("stop checking powr settings js run stuff bro"; see the 2026-10-21 run). The run used `--force --max-load 100 --wait 0` on battery. The Commander records it as the reference run for the §1.2 latency precondition of S5. All 120 gated rows pass, on main `58d6d773f` (#1875).

## Unicode M3 edit rows in the same session

These use the same driver as the 2026-10-21c and 2026-10-21d runs (`unicode/table.md`, `unicode/raw.tgz`), on the same commit. Every gated edit row meets ≤ 11 ms p95 to the edited page. b4 end measures 9.1 / 9.7 / 9.7 ms (p50 / p95 / max), and b5 EDITPOINT measures 8.2 / 8.8 / 10.0 ms.
