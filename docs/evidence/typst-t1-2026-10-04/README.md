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
`export_failed` and names it on stderr): `pdftags/issue-7257-break-tags-show-par-none`
("internal error: tags weren't properly closed", typst-pdf's tagged export). Of the 82,014 glyphs in the PDFs, **1,458 are not
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

## Paths and clips from the PDF

§15.5: "The 408 pixels (≤ 1 level) at 3× on d300 page 150 … are believed to be rules drawn
from the frame; T1 must clear them." T0's evidence (`docs/evidence/typst-t0-2026-10-04/`)
measured that they are: the frame's stroked lines sit up to 4.8 × 10⁻⁵ bp from the PDF's
numbers, and drawn from the PDF's numbers the 408 pixels disappear. The host now sends every
PATH and CLIP of a Typst page with the PDF's own CTM, segments and line state.

| Set | Paths compared | Not the PDF's | Verdict |
|---|---|---|---|
| `tests/oracle.rs` corpus | 17 | 0 | MET |
| Typst 0.15.1 test suite (2,622 snippets that compile) | 8,302 | 0 | MET |

- The checker reads the paths independently (its own operators and numbers) and requires
  each of the host's paths, in order, to be one of the PDF's bit for bit, painted with part
  of what the PDF paints (the host leaves out a gradient fill it cannot draw). A test shows
  Typst's frame paths fail it (6 of 6 on `shapes.typ` page 1).
- Run: 157.9 s for the suite (glyphs and paths) at a load average above 100.

## Latency (non-reference: not a test of the targets)

Targets (§15.3, §15.10): edited page on the socket ≤ 16 ms p95 at 100 pages, ≤ 64 ms at 300,
≤ 387 ms at 1,000. Measured end to end through the host's socket by
`typst-host/examples/typing_bench.rs`: `COMPILE` with a one-character edit sent → the first
`PAGE` frame received (the edited page), 40 keystrokes per location (12 at d1000), the first
two dropped. The machine's load average was **120–215** during every run (self-hosted CI
runners, `xctest`, other agents), 10–20× Track A's, so these numbers say how the host behaves
under that load, not whether it meets the targets: **latency is OPEN until an idle-machine run.**

| Document (pages) | Seeded: first page p50 / p95 (ms) | Seeded compile p50 | Standard: first page p50 / p95 | Load |
|---|---|---|---|---|
| d10 (9) | 33–37 / 65–98 | 12–15 | 208–273 / 327–393 | 137–208 |
| d100 (73) | 154–294 / 232–433 | 92–157 | 940–1,194 / 1,252–1,673 | 131–208 |
| d300 (217) | 692–1,320 / 1,023–1,905 | 414–841 | 1,802–2,684 / 2,515–3,192 | 155–216 |
| c300 (229) | 837–949 / 1,087–1,338 | 577–684 | — | 175–205 |
| d1000 (722) | 2,030–3,085 / 2,930–3,392 | 1,576–2,410 | — | 123–160 |

- The seeded loop ran one layout iteration on every edit (two on one d10 edit at EDITEND).
  Standard-to-seeded ratio of the first page: 3–7× at d10, 4–6× at d100, 2–2.6× at d300.
- **Where the time goes before the first page**, besides the compile: the per-page hash of
  every page (0.8 ms a page under load: 58 ms p50 at 73 pages; now hashed on up to 8 threads,
  `DONE.hash_ms`), and the glyph and path positions from typst-pdf's export of the sent pages
  (`DONE.positions_ms`: 6–10 ms p50 at d10, 15–75 ms at d100, 35–130 ms at d300 under this
  load; mostly typst-pdf's font subsetting, which grows with the fonts a page uses, not with
  the document since the export is of the sent pages alone). At Track A's load these would
  be roughly a tenth, which still leaves the 100-page target (16 ms, already met only at the
  limit by the compile alone, §15.3) without margin: an idle-machine run decides it.
- Cold compiles at this load: d10 0.26 s, d100 2.2 s, d300 15 s, c300 7.2 s, d1000 73 s to the
  first page. That run's host predates the watchdog; d1000's 73 s would have passed its first
  cold budget (60 s), so the cold budget, which also covers the idle check's standard
  compile, is now 180 s, and the app should raise it from a document's last cold time.
- Raw rows: `latency.jsonl`.

## Memory

Target: flat, ≤ 1.1 GB at 300 pages over 1,000 keystrokes.

| Document | Keystrokes | Resident memory every 50 keystrokes (MB) | Verdict |
|---|---|---|---|
| p300 (300 pages; Track A's generator, 415 sections) | 1,000 at EDITMID, all seeded, `evict(10)` after each | 1,313 → 1,342 (keystroke 250) → 1,174 → 1,082 (500) → 1,123 (1,000) | **flat: MET; ≤ 1.1 GB: NOT MET** (1,082–1,342 MB) |

- Measured as the host process's RSS (`ps -o rss`), the host holding the last document as
  the seeded loop's seed. Track A's 1 GB was macOS `phys_footprint` of an in-process loop
  without the host's per-page export; the two metrics are not the same. No growth over
  1,000 keystrokes: eviction works. Bringing the ceiling under 1.1 GB (a lower `evict` age, or
  dropping the seed when idle) is an open item. Load 118–169. Raw: `memory.jsonl`.
- For contrast, with the standard compile at d300 (217 pages) the host sat at 2.2 GB, and
  with `--verify every` (both compiles each keystroke) also at 2.2 GB.

### Eviction age (follow-up)

The host takes `--evict AGE`. On p300, 150
keystrokes each, same machine (load 114–180):

| `comemo::evict` age | RSS over 150 keystrokes (MB) | first page p50 (ms) | compile p50 (ms) |
|---|---|---|---|
| 10 (default) | 1,307 → 1,325 | 513 | 394 |
| 3 | 866 → 868 | 480 | 371 |
| 1 | 742 → 745 | 393 | 302 |

All flat; age 3 or 1 meets the 1.1 GB ceiling with no measured latency cost (the timings
are within this load's noise). **Commander ruling (2026-10-04): the default is now 3**
(DESIGN.md §13, §15.2), `--evict` stays configurable. With it the memory row is **MET**
(0.87 GB flat at 300 pages; measured over 150 keystrokes, the 1,000-keystroke run above
was at age 10). Raw: `evict.jsonl`.

## Colours from the PDF (E3, E4)

The positions gate now also compares every drawn glyph's fill colour (and, on complete pages,
its fill alpha) and every path's fill and stroke colour with the checker's, bit for bit, in two
client modes:

| Client | Snippets compiled | Glyphs | Paths | Mismatches |
|---|---|---|---|---|
| without `color-spaces`/`line-state` (ICCBased as Device colours) | 2,622 | 80,556 | 8,302 | 0 |
| with `color-spaces` and `line-state` (`--accept colour`: FILL_COLOR_CS, alpha, Separation, stroked text drawn) | 2,622 | 80,556 | 8,317 | 0 |

`tests/suite_subset.rs` adds a CI case with sRGB, grey, CMYK, alpha text and shapes, stroked
text and a spot colour: complete page, the PDF's ICC profile and Separation in COLORSPACES,
every colour and alpha equal to the checker's. Load average about 40–90 (10.8 s per run).

## Raster images from the PDF (E6)

`examples/positions_suite.rs` also compares every raster image the host draws with typst-pdf's
image `Do`s, as the independent checker reads them (its own object scan, `flate2`). The two must
be equal bit for bit, in order:
- the CTM;
- the size, components, bits, interpolation, soft mask and ICC flags;
- every IMAGE_DATA part: the samples or JPEG, the soft-mask samples and the ICC profile.

Typst 0.15.1's test suite, 2,622 snippets that compile:

| Client | Raster `Do`s in the PDFs | Images drawn | Not the PDF's | Placement or decode failures |
|---|---|---|---|---|
| `--accept colour,images` | 78 | **71** | **0** | **0** |
| `--accept colour` (no `image-data`) | 78 | 0 (44 pages INCOMPLETE: "accept image-data") | — | — |

The 7 `Do`s not drawn belong to SVG and PDF images, which are still E5 islands (24 and 4 pages).
Glyphs (80,556), paths (8,317) and boxes stay at 0 mismatches in every mode.

`tests/oracle.rs` runs the same comparison end to end through the host process:
- `raster_images_match_typst`: PNG with alpha, grey, a 16-bit PNG (which typst-pdf writes as
  8-bit), a JPEG passed through, smooth and pixelated scaling, rotated, mirrored, clipped,
  stretched and `cover`. That gives 10 images and 5 resources; an image drawn twice is sent once.
- `images_need_image_data`: without `image-data` the page is INCOMPLETE.
- `the_checker_rejects_wrong_pixels_and_matrices`: one changed sample, or a matrix one ulp off,
  is rejected.

## PDF islands (E5)

The following go as a one-page PDF:
- SVG and PDF images;
- gradient and tiling fills and strokes: shapes, text, a page fill.

Each island is a page of the page's size and bleed, holding only that item inside the same chain
of frames, groups, transforms and clips. It is drawn with the identity matrix. The independent
checker's `check_island` reads each island PDF and requires:
- its box equals the page's;
- every glyph, path and XObject `Do` its content stream shows is, in order, one the page's own
  content stream shows, bit for bit.

Typst 0.15.1's test suite, 2,622 snippets that compile:

| Client | Islands | Not the page's numbers | Glyphs / paths checked in islands | INCOMPLETE pages | Glyph, path, box, image mismatches |
|---|---|---|---|---|---|
| `--accept colour,images` | **828** | **0** | 1,466 / 754 | **0 of 3,162** | 0 |
| no `accept` | 0 | — | — | 210 of 3,162 (gradient 75, image 44, alpha 31, tiling 26, SVG 24, …) | 0 |

The suite's `images` count (899) is the 71 raster images plus the 828 islands.

`tests/oracle.rs` adds two tests:
- `islands_match_typst`, through the host process: an SVG image (twice, one of them rotated),
  linear, radial and conic gradients, a tiling, gradient and gradient-stroked text, a clipped
  gradient, and a gradient page fill. That is 9 islands, every page complete, every island the
  page's.
- `the_island_check_rejects_other_numbers`: paths one ulp off, or an island drawn with another
  matrix, are rejected.

**Belief, not measured:** that the app's drawing of an island equals the page's pixels at 2× and
3× (the DESIGN §15.4 gate row; it needs the client). Measured: the numbers are the page's.

**Effect on the latency corpus:**
- p300 with a client that accepts `image-data`: no page is INCOMPLETE, so `DONE` follows the
  first page with no `DONE.pdf` compile.
- RSS after the edits is **0.87 GB**, against 1.50–1.53 GB without islands. That meets the 1.1 GB
  gate at 300 pages.
- The 999-page runs were aborted by the load guard (load 30–55), so they are not measured here.
- Latency at that load (one-minute load 19–24, five-minute 48) is not comparable and not
  reported as a result.
