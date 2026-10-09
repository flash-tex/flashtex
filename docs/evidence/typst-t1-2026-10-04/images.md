# Raster images from the PDF (E6; 2026-10-04)

Evidence for PR #1522 (spec §11.5, DESIGN.md §15.10 T1), mac-m1max-a; exact comparisons.

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

## Review fixes

The suite was rerun after the review fixes, with the full paint comparison from colours.md:

| Client | Images drawn | Not the PDF's | Glyphs, paths | Pages INCOMPLETE for images |
|---|---|---|---|---|
| `--accept colour,images,ungated` | 71 | 0 | 0 mismatches | 28 (24 SVG, 4 PDF: E5 islands) |
| `--accept colour,images` (gate on, the default) | 71 sent | 0 | 0 mismatches | 72 (also 44 raster: pixel gate row pending) |

New CI tests in `tests/oracle.rs`:
- An EXIF-rotated JPEG (orientation 6, made here) and an ICC-tagged PNG (a profile built in the
  test, not copied). Both are placed bit for bit, and the PNG's profile is the one sent.
- `refused_and_over_budget_images_are_incomplete`:
  - an Adobe CMYK JPEG, which typst-pdf inverts with `/Decode`, makes the page INCOMPLETE;
  - an image past `--image-budget` makes the page INCOMPLETE and sends no IMAGE_DATA.
- `images_need_image_data`:
  - without `image-data`: INCOMPLETE;
  - with it, gate on: sent and INCOMPLETE;
  - with it, `--draw-ungated`: complete.

`tests/fuzz.rs` (seeded and dependency-free; proptest's `r-efi` dependency fails licence check
D3) covers:
- mutated typst-pdf exports;
- random operator sequences over the interpreter's whole vocabulary;
- image decoding at any size, depth and encoding;
- the bounded inflate.

The CI default is 2,000–3,000 cases per target. Two local runs found no panic and no allocation
past a limit:
- 200,000 cases per target, release build;
- 50,000 cases per target, debug build (`cargo test`, overflow checks on).

An earlier debug run found an overflow in the fuzzer's own test-data sizing, not in the host. It is
fixed with saturating arithmetic.
