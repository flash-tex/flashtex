# Colours, alpha, spot colours and stroked text from the PDF (E3, E4; 2026-10-04)

These numbers back PR #1513: spec §11.3 and §11.4, DESIGN.md §15.10 T1. They were measured on
mac-m1max-a. The comparisons are exact, so they do not depend on machine load.

## Checker comparison

`examples/positions_suite.rs` compares each glyph's and path's whole paint state with the
independent checker's reading of typst-pdf's export, bit for bit. The paint state is:
- the fill and stroke colour components;
- their colour spaces: Device, ICCBased with a SHA-256 of the profile bytes, or Separation with
  colorant, alternate, C0, C1 and N;
- `ca` and `CA`;
- for a glyph, the text render mode `Tr`;
- for a stroked glyph, the line state in stream space (the PDF's `w`, `d` and phase, times the
  CTM's similarity scale).

Typst 0.15.1's test suite, 2,622 snippets that compile:

| Client | Glyphs | Paths | Mismatches |
|---|---|---|---|
| without `color-spaces` / `line-state` (ICCBased drawn as the Device space; the reference normalised the same way) | 80,556 | 8,302 | 0 |
| `--accept colour,ungated` (`color-spaces`, `line-state`; gate flags off, so every page is complete and every number is compared) | 80,556 | 8,317 | 0 |

## History of the comparison

The first, narrower comparison (components and `ca` only) found 0 mismatches. Widening it, as
the review asked, found 12 Separation paths in 5 snippets that differed. The cause was the
checker, not the host: it did not decode `#20` in PDF names. With that fixed, every number
matches.

## CI tests

`tests/suite_subset.rs`:
- `colour_spaces_alpha_spot_and_stroked_text_from_the_pdf`: sRGB, grey, CMYK, alpha text and
  shapes, a spot colour, and stroked text, plain, rotated, and uniformly scaled with dashes.
  Every glyph's whole paint state and every path equal the checker's.
- `colours_without_the_capabilities_are_incomplete`: no `COLORSPACES`, none of `0x0F`–`0x13`,
  and the page INCOMPLETE for alpha, the spot colour and stroked text.
- `colours_are_gated_until_their_pixel_rows_pass`: with the capabilities and the default gate,
  the items are sent and the page is INCOMPLETE for ICC colour, alpha, separation colour and
  stroked text.
- `stroked_text_under_a_non_uniform_scale_is_incomplete`.

`src/pdfpos.rs` unit tests:
- ExtGState `/SMask` and `/BM` mark what they paint, and setting them back clears them;
- Separation defaults, `/Domain` and component counts;
- the pen scale applies to similarities only.

## Pixel gate rows (DESIGN.md §15.5): OPEN

These rows compare the app's renderer with typst-pdf's PDF at 2× and 3×:
- non-black ICC colour, on paths;
- alpha, on shapes;
- Separation;
- stroked glyphs.

They need the app's v3 renderer (`apps/mac/Sources/FlashTeXPreviewV3/DL3Renderer.swift`) to
decode and draw `COLORSPACES`, `FILL_COLOR_CS` / `STROKE_COLOR_CS`, `FILL_ALPHA` /
`STROKE_ALPHA` and `LINE_STATE`. It does none of these today: its decoder refuses opcodes
0x0F–0x13. The host therefore flags such pages INCOMPLETE ("pixel gate row pending") until the
rows pass. The existing harness (`DL3Parity.compare`, `PreviewParityTests` with
`FLASHTEX_V3_PARITY_SCALES=2,3`) can then measure them against fixtures that the host writes
with `--draw-ungated`.

T0 measured text in ICCBased colour and text with alpha at 0 differing pixels, but with the
prototype harness (`pxdiff2.swift`), not the app's renderer.
