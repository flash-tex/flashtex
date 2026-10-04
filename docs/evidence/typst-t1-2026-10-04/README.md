# Typst T1 evidence (2026-10-04)

Lane TYPST-T0T1, phase T1 of DESIGN.md §15.10: `flashtex-typst-host` (`typst-host/`).
Machine: mac-m1max-a (M1 Max, 10 cores, 32 GB), shared with other agents' builds and
benchmarks. **Every timing here is non-reference**: the load average is recorded with
each run.

## Positions checker vs typst-pdf (T1's correctness gate)

Gate: 0 mismatches on the corpus and on every Typst test-suite snippet that compiles.

| Set | Compiled | Pages | Glyphs compared | Mismatches | Verdict |
|---|---|---|---|---|---|
| Corpus: `typst-host/tests/fixtures` (text, math, shapes, links, transformed text with bleed), through the host process | 5 / 5 | 8 | 1,848 | 0 | MET |
| Typst 0.15.1 `tests/suite` (tag `9dfd3a08`), typst-dev-assets `53c12796`: 3,684 snippets | 2,622 | 3,162 | 80,556 drawn (82,014 in the PDFs) | 0 glyphs, 0 boxes, 0 pages failed | MET |

What "mismatch" means: a glyph's origin (ORIGINS) or glyph matrix (MATRIX) not
bit-identical (f64 bits) to the checker's, or a GLYPH's sp position not the sp rounding of
that origin, or a page box not bit-identical; a page whose positions the host could not
derive counts as failed.

**Exclusions** (stated, not silent): of the 2,622 snippets that compile, **1 is skipped**
because typst-pdf itself cannot export it (no PDF to compare with; the suite counts it in
`export_failed` and names it on stderr). Of the 82,014 glyphs in the PDFs, **1,458 are not
compared**: the glyphs of runs with a non-solid fill (gradient, tiling) or a spot colour, which
the host does not draw (the page is INCOMPLETE and the client draws `DONE.pdf`), and the inline
text of SVG images; the checker places them independently from the frames and leaves them
out. Every other glyph (80,556) is compared bit for bit.

- Method: `typst-host/examples/positions_suite.rs` runs each snippet through the host's
  own conversion (`convert::page` with `pdfpos`-derived positions, what a 3.3 client
  receives) and compares with `typst-host/tests/checker`, which reads the document's
  whole default export (tagged, compressed: `DONE.pdf`'s bytes) with its own object scan,
  inflate (`flate2`) and number reading. Raw summary: `positions-suite.json`.
- The 1,062 snippets that do not compile here are listed by reason in
  `positions-suite-not-compiled.json`: errors the suite tests on purpose, HTML and bundle
  targets, `@test` packages (refused until the package lock lands), the suite's native
  helpers. They are outside the gate's "every snippet that compiles".
- Run: 37.7 s for the whole suite at a load average of 104–117 (`uptime` before the run).
- Iterations that found real issues before the final run: image XObjects (an SVG or PDF
  image's text is the image's, not the page's) and SVG text drawn inline by krilla; both
  are handled and the page fails closed (INCOMPLETE) when a run is not where the PDF
  shows it.

## Seeded loop == standard compile

Gate: seeded == standard page hashes on ≥ 192 edits.

| Document | Edits (3 locations × 24) | Seeded | Equal to the standard compile | Different |
|---|---|---|---|---|
| d10 (9 pages) | 72 | 72 | 72 | 0 |
| d100 (73 pages) | 72 | 72 | 72 | 0 |
| d300 (217 pages) | 72 | 72 | 72 | 0 |
| **Total** | **216** | **216** | **216** | **0** — MET |

- Method: the host itself (`--verify every`: after each seeded compile, the standard
  `typst::compile` of the same input, page hashes compared), driven over its socket by
  `typst-host/examples/typing_bench.rs` (Track A's generator and typing: a character
  inserted before each marker, a space every 7th). Every edit's seeded compile took one
  layout iteration. Raw rows: `seeded-validate.jsonl` (load average 104–125).
- The checks are also in `typst-host/tests/seeded.rs`: 24 seeded compiles (word growth
  and an added section, which iterates as the standard compile does) all equal; and a
  document with two fixed points, where the seeded loop and the standard compile differ,
  shows both checks working: `--verify every` sends the standard pages, and the idle check
  sends them in a follow-up compile with `"cause": "verify"`.

## Watchdog

Gate: the watchdog kills and recovers a hanging plugin and a runaway `for` within budget.

| Case | Budget | Stop latency after the budget was exceeded (status 86) | New host, cold compile done | Verdict |
|---|---|---|---|---|
| WASM plugin whose function loops forever (`tests/watchdog.rs`, hand-assembled module) | 1.5 s wall | 12–27 ms (`over_ms`) | 26 ms later | MET |
| `for` over nested ranges (memory flat, RSS ceiling off) | 1.5 s wall | 30 ms (`over_ms`) | 28 ms later | MET |
| `range(200000000).map(..)` (runaway memory) | 400 MB RSS | 57–59 ms after memory was last under the ceiling (`since_under_ms`) | 31 ms later | MET |

Stack review (2026-10-04): the tests now assert the latency the watchdog itself measures from
the moment its budget was exceeded (< 1 s), not the wall time from the edit, which depended on
how fast Typst runs or allocates on a loaded machine. (The earlier `range(4000000000)` fixture
allocated an array of 4·10⁹ items and was stopped by the 4 GB RSS ceiling, not by the wall
budget; the fixture is now a loop with flat memory.) Only the compile is watched: a client that
reads nothing for three budgets gets its pages and `DONE` (`a_slow_client_does_not_trip_the_watchdog`).

- The host's own watchdog thread (`typst-host/src/watchdog.rs`) polls every 50 ms and exits
  the process with status 86 after one stderr line saying why; the client sees the socket
  close mid-compile and starts a new host (spec §11.10). Defaults: 10 s incremental, 60 s
  cold, 4096 MB. Two runs, load average about 110.
