# PDF islands (E5; 2026-10-04)

Evidence for the islands PR (spec §11.5, DESIGN.md §15.10 T1), mac-m1max-a; exact comparisons unless a load is given.

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

## With the review's pixel gate (merged from #1513 and #1522)

Islands are a gate-pending class: they are sent, and the page is INCOMPLETE until the app's
2×/3× row passes. `--draw-ungated` lifts the gate for measuring.

The suite with the full paint comparison:

| Client | Islands checked | Not the page's | INCOMPLETE pages | Mismatches |
|---|---|---|---|---|
| `--accept colour,images,ungated` | 828 | 0 | 0 of 3,162 | 0 |
| `--accept images,ungated` (no `color-spaces`) | 828 | 0 | 44 (alpha, spot colour, stroked text need `color-spaces`) | 0 |
| `--accept colour,images` (gate on) | 828 | 0 | 2,603 | 0 |

In the gated run the pending classes are:

| Class | Pages |
|---|---|
| ICC colour | 2,471 (every plain page paints in sRGB ICCBased) |
| islands | 129 |
| raster images | 44 |
| alpha | 31 |
| separation | 8 |
| stroked text | 5 |

So, until the app's gate rows pass, a client that accepts `color-spaces` gets almost every page
INCOMPLETE and shows DONE.pdf. The Mac app accepts none of these today. It gets Device colours,
and those pages are complete, as before.

The memory and latency gain measured above (no DONE.pdf compile per keystroke) holds only for a
client running ungated, or once the rows pass.

## Second review (2026-10-04)

**Colour glyphs.** typst-pdf (krilla) draws a colour glyph (SVG, COLR or bitmap data) as a
Type 3 procedure. A GLYPH's OpenType id cannot reproduce that. Before this fix such glyphs were
drawn as plain outlines, unflagged. Every glyph the PDF shows with a Type 3 font now makes its page
INCOMPLETE, gate or not: 26 pages of the suite.

**Release.** Image and island ids that no page the client holds uses are released after each
compile and rebound later (spec §5). The host keeps only a key and an id per image.
`tests/release.rs` moves a gradient on every edit, 40 times: at most one image stays bound, and
ids 1 and 2 are reused.

**Island cost.** Measured in the suite at load about 30. Islands are keyed by the island page,
place included, so an island that a reflow moves is exported again.

| Measure | Average | Maximum |
|---|---|---|
| Export time | 10.2 ms | 177 ms |
| Size | 15.3 kB | 236 kB |
