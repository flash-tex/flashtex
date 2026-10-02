# P3-ZERO-TOLERANCE: preview parity at tolerance 0 (J1, 2026-10-02)

Lane J1 `P3-ZERO-TOLERANCE` (flashtex-2a), DESIGN.md §6.2 and §12 (P3 exit:
"preview parity check at zero tolerance"). Base: `origin/main` `7fdbe0850`,
which already has #1259, #1266 and app-v3-5 (#1332). Machine: Apple M5 Pro,
macOS 26.6.2 (`raw/environment.txt`), shared with other lanes.

## What changed

- **Protocol** (`docs/protocol/display-list-v3.md` §3, §4.1, §4.2, §4.4, §4.6):
  two optional page sections and one host capability, `exact-geometry`.
  - `ORIGINS` (tag 7): one `f64[2]` per GLYPH, its origin in stream space.
  - `RULE_GEOMETRY` (tag 8): one `f64[7]` per RULE, the numbers the PDF draws
    it with (`re` rectangle, or `m`/`l` line and width, plus the CTM's
    translation).
  - An old reader skips both (§4.1 already says to skip unknown tags). A new
    reader given a page from an old writer draws from the sp positions, as
    before (the checked-in `beamer-overlays.dl3` is such a page and stays
    under test). Any count other than one per GLYPH or RULE is refused.
  - The minor number is left to the protocol owner (DESIGN.md §6.1), so
    `VERSION_MINOR` stays 2.
- **Engine** (`crates/flashtex-engine/src/displaylist/`): the interpreter
  keeps a binary64 copy of the CTM and the text matrices next to its exact
  decimal state, and writes both sections.
- **Renderer** (`DL3Renderer`): draws each glyph at its `ORIGINS` entry and
  each rule from its `RULE_GEOMETRY` entry, translating and then filling or
  stroking as the PDF does. The 0.001-grid `snap` stays only for pages
  without the sections.
- **Harness** (`PreviewParityTests`): same sweep, tolerance 0. The 1× floor
  and the Type 3 allowance are removed. `FLASHTEX_V3_PARITY_SCALES` and
  `FLASHTEX_V3_PARITY_SMOOTH` run §6.2's scale sweep and smoothing-on test.

## Why binary64, not just "exact"

Exact positions were not enough on their own. Measured at each step
(`raw/before-and-nearest.txt`):

| origins drawn from | 1× | 2× | 4× | Type 3 docs (1×/2×/4×) |
|---|---|---|---|---|
| sp + 0.001 snap (main) | 216/220 (85 px) | 220/220 | 220/220 | 3/4, 4/4, 3/4 (27 px) |
| double nearest to the exact decimal | 217/220 (97 px) | 220/220 | 220/220 | 4/4, 4/4, 4/4 |
| **the viewer's binary64 arithmetic** | **220/220** | **220/220** | **220/220** | **4/4, 4/4, 4/4** |

- **Where the misses were.** A threshold search on one remaining glyph
  (inline-math p1, X = 393.199) showed the rendering flips between two
  adjacent doubles. So the glyph sits on a pixel edge, and the last ulp
  decides which pixel it lands in.
- **What Core Graphics computes.** Two things, both found from four
  constraint glyphs and then checked on every fixture:
  - It accumulates `Td` and the advances in IEEE double.
  - It reads a PDF number as `digits × 10^-k`, so `237.283` becomes
    `237283 × 0.001`, one ulp above the nearest double.

  The exact formulas are in protocol §4.2.

## Verified (this branch, tolerance 0)

| gate | result |
|---|---|
| 83 fixtures, 1×/2×/4× | **660/660 page renders identical, 0 px** (220/220 at each scale); 30 renders drawn from the PDF (10 pages: `gs`/shading forms in beamer-madrid, beamer-visuals): **fallback rate 30/690 = 4.3 %** |
| Type 3 / TrueType / OpenType docs (`pdf-fonts-2` dl-docs: pk-bbm, pk-cm-unmapped, otf-bodoni, ttf-clearsans), 1×/2×/4× | 12/12 identical, 0 px (6 of them Type 3) |
| Scale sweep: 1–8 px/pt in 0.25 steps plus 1.14 and 2.28 (fit width @1x/@2x), 31 scales, smoothing off | fixtures **6820/6820 identical, 0 px**; font docs 124/124 identical, 0 px; fallback 310/7130 (4.3 %) |
| (same sweep with glyph `ORIGINS` only, no `RULE_GEOMETRY`) | 6749/6820: misses only at 5.0, 5.25, 6.5 and 6.75 px/pt, all thin stroked rules, max Δ 1. That is why `RULE_GEOMETRY` exists |
| Smoothing-on test (both sides), 31 scales | **not 0**: fixtures 4256/6820 identical, 40,831 px in all, max Δ 47 (1×: 130/220, 2×: 146/220, 4×: 150/220); font docs 104/124, 77 px |
| P-T2 on fixtures, display list written (`FLASHTEX_DISPLAY_LIST=/dev/null`) | **83/83** (`raw/pt2.txt`) |
| Positions checker (sp positions unchanged) | 83/83 fixtures and 4/4 font docs exact, 0 sp |
| Swift `FlashTeXPreviewV3Tests` and `FlashTeXDisplayListV3Tests` | pass; decoder parity 83/83, ORIGINS and RULE_GEOMETRY lines included |
| Rust: display-list crate, engine `displaylist` unit tests, `display_list_host` | pass |

## Not done / open

- **Smoothing stays off (the default).** With smoothing on, differences are
  a few levels on a handful of pixels per page, at glyph pixels.
  - **Hypothesis (not verified):** Core Graphics smooths a PDF text run as
    one mask, while the renderer composites each glyph separately.
  - **What was tried:** drawing runs with one `showGlyphs` call. Its
    positions are in text space, which re-rounds the origins: 0/880
    identical. Reverted.
  - **Status:** this was the second attempt; it is reported, not escalated.
- **Origins inside forms** are in form space, and the client composes the
  form matrix. This is exact on every fixture form so far, but it is not
  guaranteed bit for bit in general.
- **Display-list size:** +5.8 % over the 83 fixtures (48.3 → 51.1 MB; font
  programs dominate).

## Reproduce

```sh
cargo build --release -p flashtex-engine -p flashtex-display-list
# format as in docs/evidence/pdf-fonts-2-2026-09-30/gates/run-gates.sh (build)
python3 tools/displaylist/check_positions.py --engine $E/flashtex-initex --formats $F \
  --pool $E/pdftex.pool --dump target/release/dl3-dump --work $W -j 1
python3 tools/displaylist/check_positions.py ... --work $W2 \
  --root docs/evidence/pdf-fonts-2-2026-09-30/dl-docs
cd apps/mac
FLASHTEX_DL3_FIXTURES=$W swift test --filter PreviewParityTests        # 1x/2x/4x, tolerance 0
FLASHTEX_DL3_FIXTURES=$W2 swift test --filter PreviewParityTests
FLASHTEX_V3_PARITY_SCALES=1,1.14,1.25,...,8 FLASHTEX_DL3_FIXTURES=$W \
  swift test --filter PreviewParityTests/testEveryParityFixtureIsPixelIdenticalToThePDF
FLASHTEX_V3_PARITY_SMOOTH=1 ...                                        # smoothing on
```

Raw summaries: `raw/`.
