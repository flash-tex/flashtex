# Preview probes: constructs the parity fixtures do not draw

Owner: lane RENDER-AUDIT (Claude Code, mac-m5pro-kabir), 2026-11-01.

Each directory is a small document for one area where the Mac preview's
`DL3Renderer` could draw something other than pdfTeX's PDF: `\pdfliteral`
modes, colour stacks, `\pdfsetmatrix`, forms, raster images, PDF
inclusions, annotations, fonts, page attributes. Each case is on its own
page, so a failing page names the case.

They are **not** in the parity scoreboard's fixtures tier
(`tools/parity/parity.py`'s `FIXTURE_ROOTS`). The preview-parity workflow
(`.github/workflows/preview-parity.yml`) passes this directory to
`tools/displaylist/check_positions.py` as a third `--root`, so CI checks every
glyph and rule position (0 sp) and P-T2 against the pinned oracle, and the
zero-tolerance sweep renders each page against Core Graphics' rendering of
**pdflatex's own PDF**. There is no committed reference: every expected pixel
comes from the oracle in CI. A page flagged INCOMPLETE falls back to the PDF
and is reported as a fallback, not compared.

Locally (needs TeX Live 2026, the engine and its formats:
`scripts/engine-parity.sh --work W build`):

```sh
python3 tools/displaylist/check_positions.py --pt2 --root fixtures/preview-probes \
  --engine W/eng/flashtex-initex --formats W/fmt --pool W/eng/pdftex.pool
```

Measured on mac-m5pro-kabir with TeX Live 2026 (pdfTeX 1.40.29), 2026-11-01:
positions 10/10 exact, P-T2 10/10.

| probe | what it draws | what the display list does |
|---|---|---|
| `pv-form-state` | `\pdfxform` forms (immediate, nested, explicit black) drawn in red, blue, CMYK, a literal colour, `1 Tr`, `3 Tr`, a 0.2 bp pen | a PDF form inherits the state it is drawn in. Pages 1–8 drawn for a client that accepts `form-state` (FORMs flagged INHERITS, spec §4.3); to one that does not, pages 2–8 come INCOMPLETE (FORM_STATE). Page 9 (a pen) INCOMPLETE for both |
| `pv-stroke-text` | text render modes 0–3 (page 1); then a pen width, round/bevel joins, a miter limit, a dash, a scaled and a stretched CTM, mode 7 | page 1 drawn; the rest INCOMPLETE (no `LINE_STATE`, spec §11.4; mode 7 is text clipping) |
| `pv-literal-modes` | `page` and `direct` literals, colour and `Tc`/`Tz`/`Ts` inside text, balanced and unbalanced `q`/`Q`, a `q` left open at a page end, a clip; a string a literal shows in letters TeX typesets nowhere in the font | drawn; the literal's string INCOMPLETE (not in the font subset) |
| `pv-colorstack-pages` | xcolor red, CMYK and grey running across page breaks and a footnote, `\pagecolor`, a `\pdfcolorstackinit page direct` stack | drawn |
| `pv-matrix` | `\rotatebox`, mirroring, non-uniform `\scalebox`/`\resizebox`, raw and nested `\pdfsetmatrix` with skew, ink beyond every page edge, far-off ink brought back by a 0.001 matrix | drawn |
| `pv-images` | palette PNGs (8 and 4 bit, with gAMA 1.0, with tRNS), 1-bit and 8-bit grey, grey+alpha, RGB 8 (copied, and with gAMA), RGB 16, RGBA 16 (SMask at 8 bpc), RGBA 8; RGB, grey and CMYK JPEGs; `interpolate=true`; one `\pdfximage` drawn scaled, mirrored, squashed, rotated | drawn |
| `pv-pdf-inclusion` | included PDF pages with `/Rotate` 0/90/180/270, a CropBox off the MediaBox origin (crop and media), `/UserUnit 2`, and a page without colour operators drawn in black, red, blue and under a 4 bp pen | drawn |
| `pv-annots` | `\pdfstartlink` without and with `/Border`, hyperref's default link boxes, a `/Square` annotation with an `/AP` stream | drawn (annotations are not content; a Core Graphics reference draws none) |
| `pv-fonts` | Times through its T1 VF (ligatures, kerns), italic, SlantFont and ExtendFont from the map, small caps, microtype expansion and letter spacing, Latin Modern T1 accents | drawn |
| `pv-page-attrs` | a page with a `\pdfpageattr` transparency `/Group`, and a pdflscape `/Rotate 90` page | drawn (`/Rotate` from `\pdfpageattr` is not in the display list, and the sweep's reference ignores it too) |

Inputs: `make_inputs.py` writes the PNGs and `pv-pdf-inclusion/pages.pdf`
(plain zlib and hand-written PDF, deterministic). The JPEGs were made once on
macOS from `pv-images/rgb8.png` and are kept as made:

```sh
sips -s format jpeg -s formatOptions 90 rgb8.png --out rgb.jpg
sips -s format jpeg --matchTo "/System/Library/ColorSync/Profiles/Generic CMYK Profile.icc" rgb8.png --out cmyk.jpg
sips -s format jpeg --matchTo "/System/Library/ColorSync/Profiles/Generic Gray Profile.icc" rgb8.png --out gray.jpg
```
