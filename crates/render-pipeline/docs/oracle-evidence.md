# Oracle comparison: `flashtex-render` vs pdflatex (Latin Modern)

Narrow-case measurements over the visual-corpus harness fixtures. They do not
claim general LaTeX compatibility, pixel identity, or parity outside these
fixtures, this font, this page size and these builds. The reference engine is
a test oracle only; the product path never invokes it.

## Provenance

| item | value |
| --- | --- |
| pipeline | `crates/render-pipeline` at `1ddb43e` (branch `agent/mac-render-pipeline/unified`), `cargo build --release`, `cargo 1.99.0-nightly (3efb1f477 2026-07-17)` |
| worker invocation | `flashtex-render --secnumdepth 0 --timing`, default font search (MacTeX 2026 tree), one request per fixture, body from `\begin{document}` only (the harness strips the preamble) |
| reference engine | **MacTeX 2026 full (pdfTeX 3.141592653-2.6-1.40.29, TeX Live 2026)**, `/usr/local/texlive/2026/bin/universal-darwin/pdflatex`, `-interaction=batchmode -halt-on-error -file-line-error`; rendered fresh for this run (not reused stored references) |
| reference preamble (`pdflatex-lm`) | `\documentclass[12pt]{article} \usepackage[T1]{fontenc} \usepackage{lmodern} \usepackage[margin=1in]{geometry} \setlength{\parindent}{0pt} \setcounter{secnumdepth}{0} \pagestyle{empty}` |
| harness | `tests/visual-corpus/harness` at `origin/agent/mac-visual-oracle/reference-raster` `63f0cf59c7db28fd729400f68d3912f230aada62` (`render_reference.sh`, `render_flashtex.sh`, `diff.py`, `rasterize.swift`), run from a scratch export; 144 dpi, threshold 32, Pillow 12.2.0, Swift 6.2.4, macOS 26.3.1 arm64 |
| export side | `flashtex-pdf` from the vendored pin `4bd8c2e`, `--embed-font auto --verify`; the harness sends **no** `layout_capabilities`, so the export is the legacy v1 route (no font hints, U+2500 rule approximation) |
| generated | 2026-09-12T07:41:55Z on mac-m1max-a |

Per-fixture reference SHA-256 (fixture / pdf, first 12 hex): 01 `223aac6742bf`/`bf1058c601c0`,
02 `e4f5d58b6d47`/`b43be682f35a`, 03 `96fa8f8be5eb`/`1ec4cb2285c8`, 04 `19ba4826ae3f`/`76b5a84471e0`,
05 `3656f7c08076`/`487ec1ba605d`, 06 `f21876a4ef3d`/`a076ee8aba69`, 07 `73833d4b2399`/`ce1a7663f167`,
08 `1c7a5355db97`/`3b30c3b70cb0`, 09 `3815168adff5`/`f9dce2a3efc7`, 10 `c862912f9c40`/`39c0c54da7ec`,
11 `56b9511e1048`/`882a3916d674`, 12 `8b566fe1cee1`/`7f60ffa6af40`, 13 `7a0878277cb4`/`4d9218e1cde3`,
14 `4db4c0efbbe3`/`2b801ad873a8`, 15 `a40ef2bd5ea8`/`395c24e5e0b2`, 16 `08d443b98d6f`/`d361ac509378`,
17 `bcfad1ab8a66`/`f19c6207622b`, 18 `9a9ac05f3eca`/`112d9f3ed147`.

## Word-box and raster comparison against `pdflatex-lm`

`words` are PDFKit word boxes of both PDFs aligned by text; `dx`/`dy` are
mean/max absolute differences of the box origins in points; `line-start` is
the fraction of aligned words that agree on starting a line; `diff px` is the
fraction of differing pixels on page 1 at 144 dpi; `ms` is the worker's own
parse+layout+display-list time for the request (`--timing`, cold worker: it
includes loading the fonts for that request).

| fixture | pages ref/ours | words ref/ours (aligned) | same sequence | line-start | dx mean/max pt | dy mean/max pt | SSIM p1 | diff px | ms |
|---|---|---|---|---|---|---|---|---|---|
| 01-plain-paragraph | 1/1 | 13/13 (13) | yes | 1.00 | 0.03/0.06 | 1.15/1.15 | 0.998 | 0.23% | 1.68 |
| 02-wrapping-paragraph | 1/1 | 210/210 (210) | yes | 1.00 | 0.01/0.09 | 1.15/1.15 | 0.963 | 3.85% | 1.88 |
| 03-section-heading | 1/1 | 15/15 (15) | yes | 1.00 | 0.01/0.02 | 1.25/1.65 | 0.993 | 0.52% | 2.28 |
| 04-bold-emph | 1/1 | 10/12 (8) | no | 1.00 | 0.54/1.16 | 1.15/1.15 | 0.995 | 0.29% | 4.63 |
| 05-unicode | 1/1 | 11/11 (11) | yes | 1.00 | 0.01/0.02 | 1.15/1.15 | 0.998 | 0.20% | 1.19 |
| 06-math-inline | 1/1 | 14/17 (9) | no | 1.00 | 0.96/1.58 | 0.61/1.15 | 0.996 | 0.21% | 7.45 |
| 07-math-display | 1/1 | 13/15 (9) | no | 1.00 | 1.86/3.91 | 3.09/6.63 | 0.995 | 0.26% | 6.59 |
| 08-two-page | 3/3 | 1800/1800 (1800) | yes | 1.00 | 0.01/0.04 | 1.15/1.15 | 0.873 | 13.5% | 6.22 |
| 09-mixed-document | 1/1 | 54/57 (49) | no | 1.00 | 0.21/0.86 | 1.87/2.28 | 0.978 | 1.23% | 8.95 |
| 10-unicode-paragraph | 1/1 | 82/80 (79) | no | 1.00 | 0.42/10.77 | 0.12/0.12 | 0.978 | 1.89% | 1.90 |
| 11-nested-lists | 1/1 | 29/29 (29) | yes | 1.00 | 19.39/40.71 | 26.87/54.68 | 0.970 | 0.97% | 1.36 |
| 12-justified-paragraphs | 1/1 | 360/360 (360) | yes | 1.00 | 0.01/0.02 | 1.15/1.15 | 0.938 | 6.58% | 2.22 |
| 13-math-display-rich | 1/1 | 19/29 (11) | no | 0.82 | 8.92/27.25 | 4.43/9.73 | 0.988 | 0.43% | 6.63 |
| 14-math-inline-dense | 1/1 | 55/82 (21) | no | 0.95 | 30.54/282.01 | 4.06/14.32 | 0.981 | 0.78% | 7.06 |
| 15-three-page-sections | 3/3 | 1806/1806 (1806) | yes | 1.00 | 0.01/0.06 | 1.15/1.65 | 0.895 | 11.1% | 7.32 |
| 16-heading-page-break | 2/2 | 962/962 (962) | yes | 1.00 | 0.01/0.04 | 1.15/1.65 | 0.873 | 13.5% | 5.01 |
| 17-apostrophes | 1/1 | 25/25 (25) | yes | 1.00 | 0.02/0.06 | 1.15/1.15 | 0.994 | 0.51% | 1.34 |
| 18-ligatures | 1/1 | 36/36 (36) | yes | 1.00 | 0.22/1.49 | 1.15/1.15 | 0.987 | 0.87% | 1.42 |

Raster registration (1-D ink cross-correlation, ±60 pt search) reports a
0 pt vertical shift on every page of every fixture except 13/14 (1.5 pt, the
math fixtures below) and 11 (lists), and 0–0.5 pt horizontally.

### Reading the numbers

- **dy = 1.15 pt on every text-only fixture is not a baseline error.** It is
  constant over all words and all pages, and the ink registration shift is
  0 pt; PDFKit derives the word-box bottom from the embedded font's descent,
  and pdflatex embeds Latin Modern as Type 1 (`lmr12`) while the export
  embeds the OpenType CFF face. Heading words show 1.65 pt = 1.15 × 17.28/12.
  Baselines were checked directly: first baseline 84.27 TeX pt (`\topskip`),
  body lines 14.5 pt apart, `\section` before/after skips 40.09 pt from the
  previous baseline (TeX: 3.5ex + `\Large` baselineskip 22 pt).
- **13.5 % differing pixels on the dense pages (08, 15, 16, 12) with dx/dy at
  0.01/1.15 pt** is ink weight, not position: our export has 10 % more ink
  pixels (116 375 vs 105 111 on 08 p1) because the CFF outlines are
  rasterized without the Type 1 hinting pdflatex's output gets from the
  viewer; every word starts within 0.04 pt of the reference.
- **04-bold-emph** (8 of 10 aligned, dx 0.54): the harness export route is
  legacy v1 without `font-hints-v1`, so bold/italic segments are set in the
  regular face by the PDF writer and `bold,`/`emphasis,` split into
  `bold` + `,`. The v1 payload itself carries the styled segments; with
  `font-hints-v1` the writer selects the faces (not exercised by the harness).
- **06/07/09 math**: word alignment differs because PDFKit merges math glyphs
  differently (the export writes math italics through the legacy `?`
  fallback since Latin Modern Math is not one of the writer's faces);
  registration shift 0 pt and diff px ≤ 0.26 % on 06/07. 09's residual
  (1.87/2.28) is the same font-box offset plus the display's `?` glyphs.
- **13/14**: `\left`/`\right` and `\nu` are rejected by the compiler's math
  parser (diagnostics), so those expressions are set without them and the
  remaining words shift. Not a layout claim; reported.
- **11-nested-lists**: list environments are not implemented (`\item`
  markers become plain paragraphs) — dx 19 pt / dy 27 pt is the missing
  `\leftmargin`/`\itemsep`. Reported as unsupported in the README.
- **10-unicode-paragraph**: `ǅ` (U+01C5) has no glyph in Latin Modern
  (diagnostic `missing_glyph`); the 10.77 pt dx max is that word.
- **15-three-page-sections**: `\newpage` is an unsupported command for the
  compiler (two error diagnostics) but the page breaks are recovered from
  the source gap; all 1806 words land on the same page as the reference.

### Fixes made from run 1 (all in `1ddb43e`)

| fixture | before | after | cause |
|---|---|---|---|
| 09-mixed-document | dy 10.4/15.2 (registration −16.5 pt) | 1.87/2.28 | `\[` after a blank line took `\abovedisplayshortskip`; LaTeX's `\[` sets an empty `.6\linewidth` box first, so `pre_display_size` selects the long skip |
| 15-three-page-sections | dy 64/141 (p2/p3 shifted 55 pt) | 1.15/1.65 | `\newpage` dropped by the compiler; recovered from source as an eject penalty |
| 03-section-heading | dy 1.86/2.67 (second heading 1.0 pt low) | 1.25/1.65 | heading lines are now appended under their own `\baselineskip` (22 pt) instead of folding the difference into the before-skip, which let `\lineskip` take over |
| all with `\[`/`$$` | `(n)` equation numbers on unnumbered displays | none | the compiler counts every closed display; only `\begin{equation}` is numbered now |

## Run 2 — exact TFM metrics (pipeline `9bb7b27`, 2026-09-12T09:26:42Z)

Same harness, same MacTeX 2026 pdflatex references rendered fresh, same
measures. Between run 1 and run 2 the pipeline moved text shaping to the
`ec-lm*` TFMs (widths, kerns, ligatures, heights, `\fontdimen`s), sized
interword glue by the font at the space token, added `\/` italic
correction and TeX's space-factor rule, and moved math to TeX's metrics
(`lmmi`/`lmsy`/`lmex` = CM, embedded in math-layout; `rm-lmr*` for the
roman family) while still painting Latin Modern Math glyphs.

| fixture | words ref/ours (aligned) | dx mean/max pt (run 1 → run 2) | dy mean/max pt | ms cold |
|---|---|---|---|---|
| 01-plain-paragraph | 13/13 (13) | 0.03/0.06 → **0.00/0.00** | 1.15/1.15 | 1.26 |
| 02-wrapping-paragraph | 210/210 (210) | 0.01/0.09 → **0.00/0.01** | 1.15/1.15 | 1.93 |
| 03-section-heading | 15/15 (15) | 0.01/0.02 → **0.00/0.01** | 1.25/1.65 | 2.27 |
| 04-bold-emph | 10/12 (8) | 0.54/1.16 → **0.00/0.01** | 1.15/1.15 | 4.29 |
| 05-unicode | 11/11 (11) | 0.01/0.02 → **0.00/0.01** | 1.15/1.15 | 1.22 |
| 06-math-inline | 14/16 (12) | 0.96/1.58 → **0.00/0.01** | 1.46/2.20 | 6.89 |
| 07-math-display | 13/15 (12) | 1.86/3.91 → **0.00/0.01** | 1.27/3.00 | 6.81 |
| 08-two-page | 1800/1800 (1800) | 0.01/0.04 → **0.00/0.01** | 1.15/1.15 | 7.17 |
| 09-mixed-document | 54/56 (52) | 0.21/0.86 → **0.01/0.06** | 1.16/1.65 | 8.93 |
| 10-unicode-paragraph | 82/80 (79) | 0.42/10.77 → 0.41/10.80 | 0.12/0.12 | 1.99 |
| 11-nested-lists | 29/29 (29) | 19.39/40.71 → 19.39/40.72 | 26.87/54.68 | 1.43 |
| 12-justified-paragraphs | 360/360 (360) | 0.01/0.02 → **0.00/0.01** | 1.15/1.15 | 2.49 |
| 13-math-display-rich | 19/28 (14) | 8.92/27.25 → 10.95/29.78 | 1.00/1.15 | 6.82 |
| 14-math-inline-dense | 55/72 (36) | 30.54/282.01 → 10.72/340.11 | 1.94/18.56 | 7.15 |
| 15-three-page-sections | 1806/1806 (1806) | 0.01/0.06 → **0.00/0.01** | 1.15/1.65 | 7.97 |
| 16-heading-page-break | 962/962 (962) | 0.01/0.04 → **0.00/0.01** | 1.15/1.65 | 5.38 |
| 17-apostrophes | 25/25 (25) | 0.02/0.06 → **0.00/0.01** | 1.15/1.15 | 1.52 |
| 18-ligatures | 36/36 (36) | 0.22/1.49 → **0.00/0.01** | 1.15/1.15 | 1.40 |

Word boxes are read back from PDFs written to 0.001 bp, so 0.01 pt is the
floor of this measure. On 07-math-display the reference content stream
was decoded directly: the `\sum` glyph, both limits, the main `i =`, the
numerator, the fraction rule (x 302.386, width 43.351, thickness 0.398 bp)
and the denominator all sit at the same coordinates as the pipeline's v1
items to 0.001 bp.

Residuals, all explained: 13/14 are blocked by the compiler's math parser
(`\left`/`\right` and `\nu` are rejected and dropped, which changes the
line's natural width — the lines of 14 without those constructs are at
0.00/0.01); 11 is the unsupported list environment; 10 is the `ǅ` glyph
absent from T1/Latin Modern; the constant dy 1.15 pt is the PDFKit font-box
descent difference (Type 1 vs CFF) — ink registration stays at 0 pt.

## Metric provenance (what a clean compile guarantees)

Since `919ad8b` a compile's diagnostics state which metrics were used:

| metrics | source | when absent |
|---|---|---|
| `ec-lmr12`, `rm-lmr12/8/6` (12 pt text and math roman) | font-resources `required_tfm::RequiredMetrics` over the `texmf-dist` root, digest-bound to Latin Modern 2.004 (`REQUIRED_TFMS`: ec-lmr12 `29902112…`, rm-lmr12 `9d4e3d8e…`, rm-lmr8 `80bcbfd8…`, rm-lmr6 `eb0bfdf8…`, GUST licence `49ea6cb9…`) | **error** `required_metrics_unavailable` (text face and math roman); layout continues on OpenType metrics but is flagged as not the reference geometry |
| other `ec-lm*` text TFMs (10/17 pt, bold, italic) | shared parser from the TFM search dirs | warning `tfm_missing` naming the face |
| `lmmi`/`lmsy`/`lmex` (math families 1–3) | math-layout's embedded CM tables (byte-identical metrics to the LM 2.004 files, verified on lmmi12/lmsy10/lmex10/lmmi8/lmsy8) | n/a (embedded) |
| glyph programs | Latin Modern OTFs; `fonts[].sha256` = SHA-256 of the raw file (`lmroman12-regular.otf` `e6be218a…`) | error `font_unavailable`, `math_font_unavailable` |

An empty diagnostics list therefore means: pinned 12 pt TFM metrics, TeX
page builder, Latin Modern glyphs — the configuration the tables above were
measured in. The MacTeX 2026 files on this machine match the 2.004 digests.

## Large-edit cost: persistent worker vs fresh process (M1 Max, `4888a67`)

One-word edits in the middle of a document, Latin Modern, request and reply
over stdin/stdout including JSON; "fresh" spawns a new process per compile.

| document | pages | v1 items | persistent worker (median of edits) | fresh process |
|---|---|---|---|---|
| 08+09 bodies ×1 | 3 | 1 864 | 9.7 ms (first request 22.8 ms) | 27.0 ms |
| ×10 | 27 | 18 631 | 95 ms (first 125 ms); **71 ms at `1ade215`** (direct JSON writer) | 148 ms; 128 ms |
| ×40 | 107 | 74 841 | reply refused: 17.2 MB > 16 MiB line limit (explicit `failed`) | — |

Stage split at 27 pages (`examples/stages.rs`, `4888a67`): parse 3.5 ms,
adapt 12, typeset (shape + break + pages) 23, assemble v2 14, v1 22, JSON
12. At `1ade215` the v1 line is written directly (same bytes, no `Value`
tree): v1 9 ms, JSON 7 ms. The shaping cache now outlives requests
(`65dbe7d`), which changes little (typeset 23 → 21 ms): line breaking and
page building dominate that stage. No incremental layout reuse exists yet;
a per-paragraph cache keyed by (text, style, measure) is the next step
(adapt + typeset ≈ 32 ms of the remaining ≈ 65 ms). Found while measuring:
`FontSet::core14` re-hashed the AFM per lookup (Times documents: 25 pages
620 ms → 116 ms). Measurements were taken with other agents loading the
machine (load average 35–45); treat them as upper bounds.

## Incremental layout reuse (`a8e39c1`)

Per-block cache across requests (`src/incremental.rs`): an edit inside one
paragraph retypesets that paragraph only; every other block's lines, box
records and math boxes are cloned back with their source offsets relocated
and their diagnostics replayed. `tests/incremental.rs` compares the
`compile_result` and `display_list` lines of the caching path with a fresh
compile after each edit of a script (insert/delete words at moving
positions, change every display, delete a paragraph so later blocks shift):
**200 edits over a 27-page document (111 325 block hits / 597 misses) and 30
edits over a 107-page document (65 711 / 1 479): byte-identical throughout.**

Warm per-edit cost, one-word edit in the middle of the document, Latin
Modern, M1 Max (other agents loading the machine; the CPU column is
user+sys time of the worker per request and is load-independent):

| document | pages | before this lane (`4888a67`) | `1ade215` | **`a8e39c1`** (incremental) | worker CPU / request | fresh process |
|---|---|---|---|---|---|---|
| 08+09 ×1 | 3 | 9.7 ms | — | **7.4 ms** wall | 5.4 ms | 42 ms |
| ×10 | 27 | 95 ms | 71 ms | **~38 ms in-process** (wall medians 56–123 ms under load 13–60) | 45.6 ms (incl. request parse + 4.3 MB reply write) | 90–120 ms |

Stage split at 27 pages after the change: parse 3.4 ms, adapt 9.1, typeset
(shape + break + pages, all blocks cached) 5.4 (was 21), assemble v2 8.6
(was 13), v1 4.8, JSON 6.9. What remains is mostly output construction
for a 4.3 MB reply (assemble + v1 + JSON ≈ 20 ms) and the adapter's
per-character source records (9 ms).

### The two further levers (`0b09be57` assembled-items cache, `7ca34cec` adapter cache)

* Display items are built per block in line-local coordinates and placed
  by integer tick moves (`incremental::place_item`), the assembled lines
  cached by block key; every y tick is `baseline_tick + local_tick`, so a
  block placed anywhere yields identical ticks.
* The adapter's items for a compiler block are cached keyed by the inlines
  (kinds, texts, relative spans, label/reference keys), the source bytes
  they sit in, the style in force at the block start and the label table;
  a hit clones the items with every source record and math-atom span
  relocated.

Both keep the byte-identical gate (200 edits × 27 pages, 30 edits × 107
pages). Measured at 1-min load 14.8 (`uptime` recorded by
`scratchpad/rp/measure_quiet.sh`; still not a quiet machine — 12 lanes
were compiling), 27 pages, in-process stage timings (`examples/stages`):

| stage | `a8e39c1` | `0b09be57` | `7ca34cec` | **reply writer** |
|---|---|---|---|---|
| parse | 3.4 ms | 3.4 | 3.4 | 3.4 |
| adapt | 9.1 | 9.1 | **6.1** | 6.3 |
| typeset (cached blocks) | 5.4 | 5.4 | 5.1 | 5.6 |
| assemble v2 | 8.6 | 7.1 | 6.6 | 7.3 |
| v1 fallback | 4.8 | 4.8 | 4.7 | **1.2** |
| JSON (4.27 MB) | 6.9 | 6.9 | 6.7 | **3.8** |
| **sum** | 38.2 | 36.7 | 32.6 | **27.6** |

Persistent worker over the protocol (one-word edits, 30 per run, wall):
at `7ca34cec` (load 14.8) 27 pages median **39.7 ms** (min 38.9, max
44.3; CPU 40.4 ms per request incl. the first), fresh process 86.4 ms; 3
pages median 4.3 ms (CPU 5.2), fresh 33.3 ms. With the reply writer
(`uptime` 9:47, load 8.4 / 14.9 / 19.5): 27 pages median **33.6 ms** (min
32.6, max 36.0; CPU 34.1 ms), fresh 80.4 ms.

The reply writer change: `v1::fallback` computes one `FontHint` per font
resource instead of lowercasing the PostScript name per run (4.9 → 1.2
ms), and the envelope writer prints integers and 1/1000-pt values
directly (a rounded value below 10^11 has at most 15 significant digits,
so the milli-unit integer with trailing zeros trimmed is exactly the
shortest round-trip form `{}` printed; `scalar_fast_paths_match_fmt`
sweeps 60 000 values against `fmt`, `writer_matches_value_tree` and the
golden v1 fixtures keep the bytes). In-process the 27-page edit is now
under 30 ms; over the protocol it is 33.6 ms wall, of which ~6 ms is the
request parse plus moving a 4.3 MB line through the pipe to the harness.
"Well under 30 ms" at 27 pages therefore still needs the reply side to
shrink (page-scoped or delta replies) — a protocol change to agree with
mac-preview-v2, not a cache.

Independent check (Commander, rendering-core `ef350d2`, issue #2 11:00Z):
02-wrapping-paragraph rendered from `65dbe7d` with rooted LM 2.004 assets
and compared with Poppler at 144 DPI: all 13 line memberships and the
210-word sequence match; the earliest horizontal difference inside the
first word is +0.0054 bp, attributed to pdfTeX's `TJ`/`Tf` rounding versus
the pipeline's exact ticks (84362432 / 2^20 bp), not a layout displacement.

## rendering-v2 validation against main's `rendering-core`

`flashtex-render --v2` envelopes for all 18 fixtures (with `rules-v1` +
`font-hints-v1` requested) were run through
`crates/rendering-core/examples/validate_display` from `origin/main`
`7fea005b0611e2cc565ec38b9a705eff7cc4532c` with its
`tests/fixtures/capabilities.json`:

- as emitted: `Semantic { message: "unsupported font profile" }` on all 18 —
  the schema and validator accept only `format: "static-truetype"`, and Latin
  Modern is OpenType CFF (`opentype-cff` in our manifest). This is the one
  schema deviation; requested change in `docs/proposals/rendering-abi.md`.
- with only the `format` token rewritten to `static-truetype` (and the
  matching feature declared): all 18 report
  `status: valid_experimental_roundtrip` (canonical bytes 20 005 – 3 248 264,
  `paintable: false` as the validator always reports). Every other rule —
  tick geometry, glyph-id bounds, contiguous pages, cluster/caret/hit
  regions, source provenance against the declared documents, diagnostics —
  passes on real pipeline output.

## Latency (warm worker, M1 Max, release)

Round trip per request through stdin/stdout including JSON parse and the v1
serialization, measured from a Python driver after one warm-up request:

| request | items | v1 bytes | round trip |
|---|---|---|---|
| 01-plain-paragraph | 13 | 3 193 | 0.16–0.24 ms |
| 12-justified-paragraphs | 360 | 81 827 | 1.9 ms |
| 14-math-inline-dense | 109 | 24 429 | 0.6–0.7 ms (6.1 ms cold: math font load) |
| 08-two-page (3 pages) | 1 800 | 409 836 | 8.6–13.6 ms |

The worker's own `--timing` for the same warm requests: 0.07 / 0.92 / 0.26 /
4.22 ms. The cold first request of a process adds 5–8 ms of font loading.

## How to reproduce

```sh
# reference side (oracle only), FlashTeX side, diff — from a scratch export of the harness
export PATH=/usr/local/texlive/2026/bin/universal-darwin:$PATH
export FLASHTEX_LM_DIR=/usr/local/texlive/2026/texmf-dist/fonts/opentype/public/lm
H=tests/visual-corpus/harness   # from origin/agent/mac-visual-oracle/reference-raster
swiftc -O $H/rasterize.swift -o work/rasterize
$H/render_reference.sh --out work/reference --rasterize work/rasterize --dpi 144 \
  --texbin /usr/local/texlive/2026/bin/universal-darwin --engine pdflatex --reference-from none
printf '#!/bin/bash\nexec crates/render-pipeline/target/release/flashtex-render --secnumdepth 0 --timing "$@"\n' > work/fr; chmod +x work/fr
$H/render_flashtex.sh --out work/flashtex --rasterize work/rasterize \
  --pdf-bin crates/render-pipeline/vendor/pdf/target/release/flashtex-pdf --compiler pipeline=work/fr --dpi 144 --embed-font auto
python3 $H/diff.py --reference work/reference --flashtex work/flashtex --evidence work/evidence \
  --dpi 144 --thresholds $H/thresholds.json --rasterize work/rasterize --images-for-engines pdflatex-lm
```
