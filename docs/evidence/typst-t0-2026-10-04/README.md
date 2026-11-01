# Typst T0 exit gate, measured (2026-10-04)

Lane TYPST-T0T1, DESIGN.md §15.10 (T0 row). The spike is the Track A prototype
(`docs/evidence/typst-design-2026-09-30/prototype/`, Typst 0.15.1) plus two
harness additions in `harness/`. The other half of T0's gate, the v3.3 spec merged
with the LaTeX parity fixtures unchanged, is the v3.3 spec PR, not this one.

**Every timing here is NON-REFERENCE.** The machine is mac-m1max-a (Apple M1 Max,
10 cores, 32 GB). It was shared with other agents' Rust, Miri and Swift builds, at
a 1-minute load average of **36–161** during the runs (each raw row records it). The
design's numbers (§15.3) are from an M5 Pro at load 8–60. The ratios and the
correctness rows (hash validation, pixel counts) do not depend on load; the
absolute times do.

## Gate table

| Row | Target (§15.10 T0; §15.3 for the T1 targets) | Measured | Verdict |
|---|---|---|---|
| Seeded == standard page hashes | 0 mismatches on ≥ 192 edits | **0 / 192** (d10, d100, d300; 4 locations × 16 edits each); every seeded edit converged in 1 layout iteration | **MET** |
| p95 per size, seeded vs standard | reported per size (the T1 targets are ≤ 16 ms at 100 pages, ≤ 64 ms at 300, ≤ 387 ms at 1,000) | worst location's p95, seeded / standard: d10 33.5 / 67.3 ms; d100 201 / 417 ms; d300 500 / 1,271 ms; c300 387 / 1,158 ms; d1000 2,341 / 6,984 ms (standard d1000: 8 timed edits per location, not 38) | **Reported (MET).** Against the T1 targets: **not met on this machine**, 6–15× the M5 Pro numbers at 10–20× its load. Not a reference measurement |
| Memory with eviction | reported (T1: ≤ 1.1 GB at 300 pages) | d300 seeded, `evict(10)` after each edit: 579 MB after the cold compile, **995 / 1,006 / 1,007 MB** after 100 / 200 / 300 edits (flat). Without eviction: **7,624 MB** after 100 edits (70 MB per edit); `evict(0)` then took 5.65 s and left 1,484 MB | **MET** (flat at about 1 GB, as §15.2 says) |
| Pixel parity, ≥ 2 text pages, 2× and 3×, PDF-derived f64 origins | 0 differing pixels | d10 page 2 (3,412 glyphs) and d300 page 150 (3,627 glyphs): **0 at 2× and 3×** with PDF-derived origins and the rules drawn from the PDF's numbers, for both ways of reading the numbers (below) | **MET** |
| The 408-pixel case (d300 p150, 3×) | explained | **Explained, measured:** all 408 pixels lie within 2 px of the page's four stroked lines (three fraction bars, the footnote rule), none elsewhere. Typst's frame places those lines 1.7 × 10⁻⁶ to 4.8 × 10⁻⁵ bp from where the PDF's f32 `cm` numbers put them. Drawn from the PDF's numbers, the page has **0** differing pixels at 3× | **MET** |
| Colour (non-black text) | a gate row | 6 colours (sRGB, luma, CMYK, Oklab) on one page: **0 at 2× and 3×**, the PDF's components drawn in Device spaces and in the PDF's own ICCBased spaces alike, in an sRGB bitmap | **MET for text into an sRGB bitmap**; a display (P3) context is untested |
| Alpha (text) | a gate row | 3 paints with `ca` 0.502 and 0.302: **0 at 2× and 3×** | **MET for text**; alpha on shapes and images untested |
| Gradient island (E5) | a gate row | not measured: the harness draws glyphs and solid paths only | **OPEN** |
| Images (raster, SVG, PDF) | a gate row | not measured | **OPEN** |
| Colour glyphs | a gate row | not measured | **OPEN** |
| Variable fonts | a gate row | not measured | **OPEN** |

1× stays a documented floor (§15.5): d10 page 2 has 1,204 differing pixels at 1×
with PDF-derived origins (up to 42 levels), d300 page 150 has 0.

## What the rows mean

### Latency (`seeded.jsonl`, `standard.jsonl`, `standard-d1000.jsonl`, `validate.jsonl`)

One keystroke is `Source::edit` plus a compile, followed by `comemo::evict(10)`.
There are 40 keystrokes per location: at the start, middle and end of the
document, and backspaces in the middle. The first two keystrokes of each location
are dropped, as in Track A. "Seeded" is the 1-pass loop of §15.3 (`tbench
seeded`); "standard" is `typst::compile` (`tbench bench`, whose `compile_p95` is
the row above). Per location, as p50 / p95 / max in ms:

| doc | seeded START | seeded MID | seeded END | seeded MID-bksp | standard START | standard MID | standard END | standard MID-bksp |
|---|---|---|---|---|---|---|---|---|
| d10 | 10.9 / 18.1 / 22.7 | 12.7 / 24.1 / 27.0 | 12.5 / 21.7 / 25.6 | 12.5 / 33.5 / 60.8 | 33.9 / 61.8 / 69.6 | 34.8 / 55.1 / 64.9 | 39.4 / 55.0 / 64.9 | 31.7 / 67.3 / 89.6 |
| d100 | 104 / 174 / 214 | 133 / 201 / 212 | 110 / 179 / 197 | 106 / 154 / 181 | 289 / 417 / 512 | 264 / 353 / 449 | 227 / 331 / 414 | 221 / 311 / 349 |
| d300 | 361 / 500 / 528 | 296 / 414 / 498 | 273 / 380 / 428 | 252 / 376 / 426 | 727 / 926 / 1,017 | 954 / 1,271 / 1,312 | 739 / 988 / 1,078 | 740 / 977 / 1,059 |
| c300 | 245 / 331 / 400 | 273 / 387 / 463 | 242 / 351 / 470 | 238 / 349 / 374 | 648 / 857 / 882 | 597 / 1,089 / 1,173 | 856 / 1,153 / 1,537 | 852 / 1,158 / 1,540 |
| d1000 | 742 / 1,823 / 2,335 | 703 / 795 / 916 | 1,092 / 1,913 / 2,689 | 1,410 / 2,341 / 2,735 | 6,366 / 6,984 | 4,569 / 5,359 | 4,483 / 5,196 | 3,815 / 5,138 |

The d1000 standard cells are p50 / p95 over 8 edits, so p95 equals the maximum.
Load averages ran from 82 to 161 during these runs.

The standard/seeded ratio of the worst location's p95 is 2.0–3.0× (Track A, §15.3: about 2–3.8×).
The cold compiles at this load were d10 0.20 s, d100 4.3 s, d300 6.8 s, c300 3.1 s
and d1000 45 s, against 1.6 s for d300 in Track A.

The validation run re-ran the standard compile after every seeded edit and
compared every page's `hash128`. It found 0 mismatches in 192 edits: d10, d100
and d300, 16 edits at each of the 4 locations.

### Memory (`mem.jsonl`, `mem-noevict.jsonl`)

`tbench mem` on d300 used seeded compiles at the 3 locations in turn. The phys
footprint plateaus at about 1.0 GB with `evict(10)`. Without eviction it grows by
about 70 MB per edit. The edit latencies inside `mem.jsonl` are under the same
load and are not used.

### Pixel parity (`pixel-parity.jsonl`)

`harness/pxdiff2.swift` renders typst-pdf's page with Core Graphics
(`drawPDFPage`) and draws the same page with Core Text from a description. Both go
into an 8-bit sRGB bitmap with font smoothing off, antialiasing on and subpixel
quantisation off. The description is drawn as display-list-v3 §4.2/§11.2 tells a
client to draw it: in stream space (bp, y up), each glyph at its origin (X, Y),
from the original OpenType font files Typst used. Every pixel is counted whose
largest channel differs. Each differing pixel is also classed as within 2 px of a
path's bounding box, or elsewhere.

Variants:

- **Glyph origins.** `frame` uses Typst's own frame positions. `nearest` and `cg`
  are re-derived from typst-pdf's content stream (`harness/pdfpos2.py`). `nearest`
  reads every number as the double nearest its decimal, as Track A's `pdfpos.py`
  does. `cg` uses spec §4.2's viewer arithmetic: a number with k fraction digits
  is m × double(10⁻ᵏ), and products are row by column, sums left to right. The two
  differ in 1,558–1,803 glyph origins per page, by ulps.
- **Rules and lines.** `frame` draws them from Typst's frame, which is what
  `pxdiff.swift` does. `pdf` draws them from the PDF's `cm`, `w`, `J`, `j` and path
  operators.

Results:

| page | scale | frame origins | nearest | cg | nearest/cg with frame shapes |
|---|---|---|---|---|---|
| d10 p2 | 2× | 79 | **0** | **0** | 0 (the page's 7 lines draw identically) |
| d10 p2 | 3× | 5,323 | **0** | **0** | 0 |
| d300 p150 | 2× | 43 | **0** | **0** | 0 |
| d300 p150 | 3× | 0 with PDF shapes, 408 with frame shapes | **0** | **0** | **408**, all within 2 px of a line |

The original prototype harness (`pxdiff.swift` + `pdfpos.py`, y-down, frame
shapes) was re-run on the same pages and reproduces Track A exactly: 0 / 0 on d10
page 2 at 2× / 3×, and 0 / 408 on d300 page 150.

The 408-pixel case is therefore explained by measurement. typst-pdf (krilla)
writes each line's position in f32: `1 0 0 -1 291.11957 424.87564 cm`, where
Typst's frame has 291.11956194… and 424.87565827…. At 3× that moves an
anti-aliased edge row by one coverage level (≤ 1 level, 408 pixels). It is not a
glyph effect: with frame shapes the count is the same 408 for all three glyph
sources, and with PDF shapes it is 0 for all of them. **T1 consequence:** rules
and paths must come from the PDF's numbers too, as glyph origins do. This is
§4.4's `RULE_GEOMETRY` idea, applied to Typst's `PATH`s.

### Construct classes (`constructs.jsonl`)

`docs/colour/main.typ` has one A4 page of text in six fills: `rgb("#1f6feb")`,
`rgb("#d1242f")`, `luma(40%)`, `cmyk(80%, 10%, 0%, 20%)`, `rgb("#2da44e")` and
`oklab(60%, 0.1, -0.1)`. typst-pdf writes them in an ICCBased RGB space (`/c0`, N 3),
an ICCBased grey space (`/c1`, N 1) and DeviceCMYK. `docs/alpha/main.typ` has text
with alpha 50% and 30%, written as ExtGStates with `ca` 0.5019608 and 0.3019608.
Positions are `cg`, and the colour is drawn three ways:

- `device`: the PDF's components in DeviceGray, DeviceRGB or DeviceCMYK, times
  `ca`. This is a display list without E3.
- `icc`: the same components in the PDF's own ICC profile. This is E3.
- `frame`: the prototype's `to_vec4_u8` as sRGB. This variant is wrong for luma,
  CMYK and Oklab by construction, because it takes the colour's own components;
  it is a harness artefact, recorded only to show the variant matters.

| page | 2× device | 2× icc | 3× device | 3× icc |
|---|---|---|---|---|
| colour | **0** | **0** | **0** | **0** |
| alpha | **0** | **0** | **0** | **0** |

Gradient islands, images, colour glyphs and variable fonts are **open**. The
harness draws only glyphs and solid paths, and the dump does not carry variation
coordinates. Each needs host or app code that does not exist yet:

- islands (E5): a per-construct typst-pdf export and an IMAGE `pdf` item;
- images (E6): `IMAGE_DATA` with the PDF's samples;
- colour glyphs: an island, or the font's COLR/sbix through Core Text;
- variable fonts: the instance's variation coordinates (E1 `variations`) applied
  by Core Text against krilla's instanced subset.

These are T1 and T2 rows.

## Verified and believed

**Verified (measured here):**

- Seeded == standard on 192/192 edits.
- Eviction keeps d300 flat at about 1.0 GB, and growth is about 70 MB per edit
  without it.
- 0 differing pixels at 2× and 3× on two text pages, with PDF-derived origins and
  PDF-derived rules.
- The 408 pixels are the four lines drawn from frame coordinates.
- Coloured and alpha text are pixel-identical at 2× and 3× in an sRGB bitmap,
  whether drawn in Device spaces or in the PDF's ICC spaces.
- The `nearest` and `cg` readings of the numbers both give 0 pixels on these
  pages, so these pages cannot tell them apart.

**Believed, not shown here:**

- On an idle M5 Pro the latencies are about Track A's (§15.3).
- Colour parity holds in the app's display colour space as well as in sRGB.
- Islands, images, colour glyphs and variable-font instances can be made exact.
- Spec §4.2's `cg` arithmetic is the right one for Typst PDFs, since it is the one
  measured on LaTeX fixtures.

## Reproduce

The scripts expect a work directory W that holds:

- `proto-copy/`: a copy of the prototype directory, with `harness/pdfpos2.py` and
  `harness/pxdiff2.swift` copied into `proto-copy/px/`;
- `target/`: `CARGO_TARGET_DIR` for building `proto/`, which produces
  `target/release/tbench`;
- `venv/`: a Python 3 venv with `pikepdf`, `fonttools`, `numpy` and `Pillow`;
- `harness/*.sh`, copied into W itself.

Then, in W:

```sh
bash gen.sh            # d10 d100 d300 d1000 c300
bash lat.sh            # validate, seeded, standard, memory -> raw/*.jsonl
bash dump.sh           # page dumps + PDFs, builds W/pxdiff
(cd proto-copy/px && ../../venv/bin/python pdfpos2.py d10/p2.json d10/p2.pdf 2 d10/p2-pos2.json \
  && ../../venv/bin/python pdfpos2.py d300/d300p150.json d300/d300p150.pdf 150 d300/p150-pos2.json \
  && swiftc -O pxdiff2.swift -o ../../pxdiff2)
bash px.sh             # raw/pixel-parity.jsonl
mkdir -p proto-copy/docs/colour proto-copy/docs/alpha   # copy docs/colour, docs/alpha from here
bash constructs.sh     # raw/constructs.jsonl
```

The corpus generator is deterministic. The page dumps, PDFs and font blobs are
build outputs and are not committed.
