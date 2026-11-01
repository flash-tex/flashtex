# Unicode M3: the §1.2 wall-time rows on the reference Mac (2026-10-21)

Lane UNICODE-WALL (Commander `kabir-claude`). These are wall-time measurements of
Unicode M3's §1.2 rows, criterion 3 of
[`../unicode-m3-2026-10-11/README.md`](../unicode-m3-2026-10-11/README.md). They
complete that README's rows 3 and 7, which were waiting on the reference Mac. The
measurement code was not changed. The only change is a docstring note in
`tools/unicode-m3/latency.py`.

- **Machine:** Apple M5 Pro, 15 cores, macOS 26.6.2. This is the owner-approved
  reference machine for this lane. Runs used plain priority (no `nice`), and nothing
  else was building or testing. DESIGN.md §1.2 (2026-10-06) names the *slowest
  supported Mac (M1-class)* as reference hardware, so an M1 would show larger
  numbers.
- **Binaries:** `flashtex-host-unicode` (11,935,008 bytes, SHA-256 `19af851e…`),
  `flashtex-xetex` and `dl3-client`, built from main `5439b97ea` the way
  `unicode-m3.yml` builds them. `tools/` is the same on main `50318f057`, where
  this PR is based.
- **TeX Live:** `/usr/local/texlive/2026`. `FLASHTEX_FONT_DIRS` is set to its
  `fonts/opentype:fonts/truetype`, as `sweeps-unicode.yml`'s latency job sets it.
  Without it, b2's TeX Gyre Termes is not found by name and b2 ends with status
  `error` (see `raw/latency-nofontdirs.*`).
- **Quantity:** the client's wall clock, from COMPILE written to the frame read.
  For the edit rows this is the gated **host** share (DESIGN.md §1.2, decision 1:
  COMPILE written → edited-page frame read, ≤ 11 ms p95). The app's ≤ 4 ms is not
  measured here.

## Verdict per row

| §1.2 row | bench | measured (this Mac) | target | met? |
|---|---|---|---|---|
| Edit, keystroke at EDITPOINT | b1 article | edited page p50 6.2, **p95 7.8**, max 9.2 ms | host ≤ 11 ms p95 | **met** |
| | b2 fontspec, by name | p50 1.7, **p95 2.5**, max 3.3 ms | host ≤ 11 ms p95 | **met** |
| | b3 unicode-math | p50 2.9, **p95 3.6**, max 4.7 ms | host ≤ 11 ms p95 | **met** |
| | b4 fontspec, 19 pages | p50 4.6, **p95 4.8**, max 8.4 ms | host ≤ 11 ms p95 | **met** |
| | b5 about 300 pages (327) | p50 8.7, **p95 9.5**, max 12.8 ms | host ≤ 11 ms p95 | **met** |
| Edit, keystroke before `\end{document}` | b4 | p50 14.0, **p95 14.6**, max 14.7 ms (each resumes from a page checkpoint, `starts: page`) | host ≤ 11 ms p95 | **not met** (3.6 ms over) |
| (also measured) | b5 | p50 34.2, **p95 35.3**, max 40.3 ms | host ≤ 11 ms p95 | **not met** |
| Preamble edit | b1–b5 | first page 52.1 / 118.6 / 305.9 / 127.5 / 150.5 ms. Earlier run without font dirs: 51.6 / 115.5 / **347.9** / 130.9 / 165.3 | ≤ 400 ms to the first page | **met** (max 347.9 ms, b3) |
| Reopen in a new host from the persisted S₀ | b1–b5 | page 1 after COMPILE: 20.1–42.7 ms. Host start to HELLO: median 66.9 ms (min 36.8). Belief, a sum of two measurements: spawn → page 1 is 87–110 ms | ≤ 100 ms to the first visible page | **report-only**, as in Classic T7 (see below). The host's share is ≤ 100 ms except b1 at EDITPOINT (≈ 110 ms) |
| Classic T7 latency (criterion 7) | Classic corpus | PR [#1863](https://github.com/flash-tex/flashtex/pull/1863), main `5439b97ea`, M5 Pro | T7's gated rows | **118 of 120** gated rows met (cited, not re-run) |

### Notes on the rows

- **Edit row, b4's end:** the 10-20 PC count was 545 M instructions. On this Mac
  the keystroke takes 14.0–14.7 ms. That is under the whole 16 ms row, but over
  the host's 11 ms share, and the 16 ms gate is key → preview commit, which
  includes the app's ≤ 4 ms. So by the owner's split the row is **not met**. The
  cause the 10-20 README gives still holds: the pages after the `\lipsum` peek do
  not converge (READ-TRACK.md). b5's end (35 ms) fails for the same reason.
- **Preamble row:** each `latency.py` run makes one preamble edit per document, in
  a running host, after the edits. It is a full run from the format, which is the
  §1.2 definition. So there are two samples per document and no p95.
- **Reopen row, which target:** §1.2 says "≤ 100 ms to first visible page (stored
  pages, shown stale until the host confirms; a pre-warmed host does not count)".
  Classic T7 (`tools/incr-bench/t7.py`, decision 8) makes reopen **report-only**,
  because the row is met by the app's stored pages and a host-socket harness
  cannot see them. It reports the host's share of a cold reopen. The same applies
  here. `unicode.py --reopen` times from COMPILE in the new host, which excludes
  the process start, so host start to HELLO was measured separately
  (`tools/unicode-m3/host_startup.py --runs 10`) and added. That sum is a belief,
  not one measured interval.
- **latency.py's reopen (66–312 ms)** comes from a host with no S₀ cache, which
  makes a full cold run. It predates the persisted S₀ (#1805), so it is not the
  reopen row. The same is true of latency.py's "5 edits" (87–348 ms). Those are
  full runs through `dl3-client`, not the edit row. Neither is used for a verdict
  above.
- **Page counts:** `latency.py` reports b4 as 38 pages and b5 as 654. Those are
  pages summed over both passes. `unicode.py` reports the document's own 19 and
  327 pages.
- b2's cold compile at EDITPOINT (704 ms) was the first run against the new font
  dirs, which builds the font index. It is not a row.

## Procedure (reproducible)

`raw/run.sh` does the following:
- copies `tools/unicode-m3/bench/*.tex`;
- writes `b5-long.tex` from `latency.py`'s `LONG`;
- makes one unmeasured warm-up compile, which builds the format cache;
- for each bench at `EDITPOINT` and at `end`, runs:

```
python3 tools/incr-bench/unicode.py HOST DOC.tex --doc NAME --keys 20 --at EDITPOINT|end --reopen \
  --host-env FLASHTEX_S0_CACHE=DIR/s0-NAME-AT --out lat/NAME-AT.jsonl
```

This is the invocation `sweeps-unicode.yml`'s latency job uses, without callgrind
and with 20 keystrokes instead of 3. Each keystroke is a letter typed at the same
place, as the app sends it. Every keystroke waits for the S₀ save of the compile
before it. `raw/summ.py` turns the JSONL files into `raw/edits-summary.md`
(p95 = the 19th of 20 sorted).

Preamble: `latency.py --engine … --host … --client … --runs 5` with the TeX Live
font dirs (`raw/latency-fontdirs.*`), plus the earlier run without them
(`raw/latency-nofontdirs.*`).

## Raw files

- `raw/edits/*.jsonl`: one line per compile (cold, key1–key20, reopen), from `unicode.py`.
- `raw/edits-summary.md`: the per-bench table.
- `raw/latency-fontdirs.{txt,json}` and `raw/latency-nofontdirs.{txt,json}`: `latency.py --runs 5`.
- `raw/host-startup.json` (no env) and `raw/host-startup-env.json` (the bench env): `host_startup.py --runs 10`.
- `raw/run.sh`, `raw/summ.py`: the driver and the summary.
