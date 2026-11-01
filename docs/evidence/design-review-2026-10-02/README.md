# Design review 2026-10-02: measurements

For [`docs/design/engine-v2/reviews/2026-10-02.md`](../../design/engine-v2/reviews/2026-10-02.md) §3.

- **Host:** mac-m1max-a, Apple M1 Max, 10 cores, macOS (Darwin 25.3.0), MacTeX 2026
  (`/usr/local/texlive/2026/bin/universal-darwin` first on PATH; pdfTeX 1.40.29).
- **Code:** `origin/main` `ab9893935` (#1375), release build.
- **Settings:** `CARGO_BUILD_JOBS=4`, `--jobs 3`, `nice -n 5`, `SOURCE_DATE_EPOCH=0
  FORCE_SOURCE_DATE=1` (set by the script).
- **Load:** the Mac was shared with other agents' builds. The 1-minute load average was
  22 at the start, 51–138 during lockstep, and 23–34 during parity, trip and etrip
  (`raw/env.txt`; spot checks with `uptime`). Wall times below are therefore high, but
  none of the gates measured here depends on time.
- **Commands** (from the repository root):
  ```
  scripts/engine-parity.sh --jobs 3 --work <WORK>/ep build lockstep parity
  scripts/flashtex-trip.sh
  scripts/flashtex-etrip.sh
  ```

| gate | result | wall |
|---|---|---|
| build (`flashtex-initex`, `pdflatex.fmt`, `pdftex.fmt`) | ok | 137 s |
| T1 lockstep (`tools/lockstep`, every case) | **1,435 cases, 1,435 equal, 0 differ**; non-gating accounting: 7 cases differ (`1410-tracingstats` and `2130`–`2135` strings cases, "memory usage") | 719 s |
| P-T1 / P-T2, parity fixtures tier | **83 measured, P-T1 83/83, P-T2 83/83**; L0–L2 100 %, L3 98.8 %; non-gating accounting: 83/83 documents differ in the end-of-run block P-T1 leaves out | 394 s |
| trip | **pass**: tripin.log, trip.log, tripos.tex byte-identical; tripin.fot, trip.fot identical after the accepted differences; trip.typ identical after DVItype's banner | 14 s |
| etrip | **18/18 PASS** against pdfTeX's own output | 53 s |

Not measured: the arXiv and templates tiers, T2, T4, soundness and every latency benchmark
(they need hours or a quiet host; §14 puts heavy runs on the NixOS PC).

**Belief, not verified:** the fixtures' accounting difference on 83/83 is the constant
`strings out of` maximum gap (35 strings / 885 characters) that #1365 left, as flashtex-2a
reported on #1315; this run did not separate the lines.

Raw output (local paths replaced by `<WORK>`, `<TMP>`, `<REPO>`): `raw/engine-parity.txt`
(the build and step log without per-crate and per-fixture lines), `raw/lockstep.txt`,
`raw/parity-fixtures-report.md`, `raw/trip.txt`, `raw/etrip.txt`, `raw/env.txt`.
