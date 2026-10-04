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
