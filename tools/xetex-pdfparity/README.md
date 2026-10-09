# XeTeX PDF parity harness

The PDF gate of Unicode mode (`docs/design/xetex/PLAN.md` §3.5, item 4 of
§4A's S2 part 2 note): a candidate engine's PDF against `xelatex`'s, compared
**structurally and visually, not byte for byte**. Font subsets, object order
and compression legitimately differ between FlashTeX's writer and
xdvipdfmx. TeX Live 2026's `xelatex` (`/Library/TeX/texbin`) is the oracle
only and never runs in the product path (DESIGN.md).

```sh
python3 tools/xetex-pdfparity/run.py --engine <bin> [--cases 'd0*'] [--jobs N] [--keep] [--json OUT]
python3 tools/xetex-pdfparity/run.py --self-test        # xelatex against xelatex, and against qpdf rewrites
python3 tools/xetex-pdfparity/run.py --engine <bin> --baseline tools/xetex-pdfparity/baseline.json   # the CI gate
python3 tools/xetex-pdfparity/run.py --engine <bin> --write-baseline FILE [--label TEXT]
python3 tools/xetex-pdfparity/compare.py REF.pdf CAND.pdf [--json OUT] [--diff-dir DIR]
python3 -m unittest discover -s tools/xetex-pdfparity -p 'test_*.py'
python3 tools/xetex-pdfparity/xdvmeasure.py             # xdvipdfmx's precision (below)
```

Common options: `--tol` (bp, default 0.01), `--rel-tol` (em, default 0.005),
`--width-tol` (glyph-space units, default 1),
`--scale` (raster px per bp, default 2), `--no-visual`, `--smooth`,
`--diff-dir`, `--examples N`, `--glyph-identity gid|outline`. `run.py
--formats DIR` keeps the two `xelatex.fmt` files for later runs. Building a
format takes about 65 s here. Rebuild it whenever an engine changes.
`run.py --xdv-tol` (bp, default 0.001) sets the XDV check's tolerance, and
`--no-xdv-check` skips that check.

Needs: Python 3 with fontTools, Pillow and pyobjc's Quartz (all already
installed on mac-m1max-a), and `qpdf`. qpdf parses the PDFs
(`--json=2 --decode-level=specialized`); this harness only walks its
objects.

## Runner

For each case `.tex` (all of `tools/xetex-lockstep/latex-cases/`, l001-l010,
referenced in place, plus `cases/`), each engine builds its own `xelatex.fmt`.
It then runs the case in its own directory. Both steps import
`tools/xetex-lockstep/latex.py` (`build_format`, `link`, `env_for`,
`RUN_ARGS`) rather than forking it: the same `xelatex` symlink, the same
`SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1 TZ=UTC` environment, and the same
arguments **without `-no-pdf`**. TeX Live's xetex therefore writes the PDF
through xdvipdfmx. The FlashTeX binary, invoked the same way, is to write
`<job>.pdf` itself; that mode is being built separately.

The reference runs until its auxiliary files (`.aux .toc .out .lof .lot .nav
.snm .bbl`) stop changing, at most `--max-passes` (4). The candidate then
runs the same number of passes. A case fails when the reference does not
exit 0, when the exit statuses differ, when a PDF is missing, when the
comparison finds a difference, or when the XDV check fails.

**The XDV check** is what shows the writer is right. FlashTeX's writer places
glyphs at TeX's exact positions (lane decision, 2026-10-09). After its PDF
passes, the reference runs once more with latex.py's own `-no-pdf`
arguments in the same directory. The auxiliary files are stable by then, so
it typesets the same pages, and its XDV holds TeX's exact positions. Every
glyph the candidate's PDF draws itself (not inside an included PDF) is
paired with an XDV glyph by glyph id and nearest position within 1 bp, not by
order. Each pair must agree within **`--xdv-tol` 0.001 bp**. The XDV is read
with its num/den/mag, with the origin 1 in, 1 in from the top left of the
MediaBox, as xdvipdfmx does.

Glyphs placed inside transformations that specials set up (TikZ nodes,
`\rotatebox`, `\scalebox`) are placed by interpreting those specials as the
writer does (`precision.Specials`, following #1712's `out/special.rs` and
`out/content.rs`):

* a CTM with its own save stack;
* `cm`, `q` and `Q` in `pdf:code`/`pdf:direct`/`pdf:literal direct`;
* `pdf:literal`, which moves the origin to the current point and back;
* `pdf:content`, the same inside a save and restore;
* `pdf:bcontent`/`pdf:econtent` with their coordinate stack;
* `pdf:btrans`/`pdf:etrans` (scale, xscale, yscale, rotate, matrix) and
  `x:scale`/`x:rotate`, about the current point;
* `x:gsave`/`x:grestore`.

A glyph of the candidate's page with no XDV glyph within 1 bp fails the
check, as does a pair more than 0.001 bp apart. In the self-test the
candidate is xdvipdfmx's own PDF, so the check measures xdvipdfmx (below)
and does not gate. That also tests this interpretation independently:
every glyph of xdvipdfmx's PDF, including d03's 27 TikZ glyphs and d05's
rotated and scaled ones, is paired within xdvipdfmx's own error.

**Baseline (`--baseline FILE`, the CI gate).** The baseline is a JSON file
listing, per case, the differing pixels allowed at its `scale` and the
ToUnicode differences allowed (glyph, reference text, candidate text). With a
baseline, a case passes when all of these hold:

* the run is clean: exit statuses, PDFs, and the XDV check (every glyph
  paired, within 0.001 bp);
* every structural difference is one of the case's listed ToUnicode
  differences, so every glyph matches under the `0.01 bp + 0.005 em` ruling;
* the differing pixels are at most the baseline's.

`run.py` exits 0 only when every case passes. A case missing from the
baseline allows nothing. Listed ToUnicode differences that no longer occur
are reported, so the baseline can be tightened. `--write-baseline FILE
[--label TEXT]` writes a run's pixels and ToUnicode differences.
Differences of any other kind are recorded there under `not_allowed`, and
the gate never allows them.

## What is compared (`compare.py`)

* **Pages**: count; MediaBox, CropBox (defaulting to MediaBox), and
  Bleed/Trim/ArtBox when present, all inherited through `/Parent`; Rotate.
* **Glyphs**, per page, from a content-stream interpreter (`content.py`) that
  also runs the form XObjects a page paints. For each glyph: the font's
  PostScript name with the 6-letter subset tag stripped, the glyph identity,
  the origin, the linear part of the text rendering matrix, the render mode,
  fill colour (and stroke colour and width in stroking modes) and opacity.
  Glyphs are paired by identity and nearest position. Paired glyphs must
  also keep the reference's content order (`glyph-order`). Leftovers are
  reported
  as `glyph-position` (same glyph within 5 bp, with the offset),
  `glyph-id`/`glyph-font` (a different glyph at the same place; "same
  outline (renumbered)" when the outlines agree), or
  `glyph-missing`/`glyph-extra`.
* **Glyph identity** (`fonts.py`). Both rules below are VERIFIED against the
  font files (2026-10-09; see Measurements).
  * Type0 `CIDFontType2`: the glyph is `/CIDToGIDMap` applied to the CID.
    `/Identity` gives the CID; a stream gives its 2-byte entry.
  * Type0 `CIDFontType0` with a CID-keyed CFF: the glyph is the CID. The
    CFF charset is used only to find the program for the outline. A
    name-keyed CFF is indexed by the code.
  * Simple fonts: the glyph name from `/Encoding` and `/Differences`, else
    the embedded program's built-in encoding (CFF `Type1C` or Type 1
    `FontFile`, read with fontTools), else the code.

  Every glyph also gets an **outline digest**: the decomposed outline drawn
  from the embedded program, rounded to 1/100 unit. `--glyph-identity
  outline` pairs glyphs by digest instead of id, so a writer that renumbers
  its subsets compares equal.
* **Per font and glyph used**:
  * the subtype and CIDFont subtype (`font`);
  * the advance width from `/W` or `/Widths`, within `--width-tol` (1
    glyph-space unit; xdvipdfmx writes integers, for example CMMI10 `a` as
    528 against 528.59) (`glyph-width`);
  * the ToUnicode text (`tounicode`);
  * the outline digest (`glyph-outline`).

  The embedded program's kind is only a **note**, outside the difference
  total. PLAN §3.5 names a font by PostScript name, face and glyph id, so a
  TFM font embedded as Type 1 `/FontFile`, where xdvipdfmx writes
  `/FontFile3 /Type1C`, is not a parity difference.
* **Rules**: a filled axis-aligned rectangle (`re f`, or TikZ's
  `m l l l h f`) and a butt-capped, undashed, axis-aligned stroked segment
  (xdvipdfmx's `q .3985 w x0 y m x1 y l S Q` for thin rules) both count as
  the rectangle they cover. They compare equal when their colours match and
  every edge agrees within `--tol`.
* **Other paths**: painting operator, clip operator (`W`/`W*`), the
  segments with every point transformed to page space (within `--tol`),
  fill and stroke colour and opacity, blend mode, soft mask, and line width,
  dash, cap, join and miter scaled to page space. Moves that draw nothing
  are dropped first.
* **Images**: the CTM at `Do` (the unit square's placement, within
  `--tol`), pixel size, colour space (resolved: ICC by profile digest,
  Separation, Indexed, ...), bits, image mask, soft mask, `/Decode`, and a
  digest of the data as qpdf decodes it. Flate/PNG predictors are decoded,
  so a re-compressed PNG compares equal; DCT is compared as stored. Inline
  images are included.
* **Forms** (included PDF pages): the CTM at `Do` times `/Matrix`, `/BBox`,
  `/Group`. Their content counts as the page's glyphs, paths and images.
  **Shadings** (`sh`): the CTM and the shading's whole dictionary.
* **Shadings, patterns, functions, soft masks, a form's `/Group`, Separation
  and DeviceN tint functions** are compared **structurally**, with numbers
  as values within `--tol` (`0.0` equals `0`, `100.00128` equals
  `100.001`). They use the canonical form with references resolved. Streams
  that are text (a tiling pattern's cell, a form, a PostScript calculator
  function) are compared as token lists, so whitespace, comments and number
  formatting do not count; other streams (sampled functions, ICC profiles)
  are compared by a digest of their data. A difference names the first
  path that differs, for example `/Function/C0[0]: 1 vs 0.9`.
* **Links**: rect (within `--tol`); action (`URI`, `GoTo` by name, also
  resolved to page/kind/coordinates, explicit destinations); border, `/BS`,
  `/C`, `/H` and `/F`. **Other annotations**: subtype, rect, and a canonical
  digest of their dictionary.
* **Named destinations**: name to page, kind and coordinates, from the
  `/Names` tree and from `/Dests`.
* **Outline**: every entry in order, with depth, title, target, open or
  closed, flags and colour.
* **Document information**: Title, Author, Subject, Keywords and Creator.
  Producer, CreationDate and ModDate are ignored.
* **Catalog**: PageMode, PageLayout, OpenAction, ViewerPreferences and
  PageLabels.
* **Visual** (`raster.py`): every page of both PDFs is rasterised by Core
  Graphics with the app's preview-gate settings
  (`DL3Renderer.bitmapContext` and `rasterize(pdfPage:)`): 8-bit RGBA
  premultiplied-last sRGB, `ceil(w×scale)×ceil(h×scale)`, a white ground,
  antialiasing on, font smoothing off, subpixel positioning on, MediaBox
  origin, `CGContextDrawPDFPage`. A pixel differs when any of its four bytes
  differs. The report gives the count per page (0 is the target), the bbox
  and the largest channel delta. `--diff-dir` writes the reference with
  differing pixels in red, plus both renders.

**Tolerances**:

* Positions: `--tol`, default 0.01 bp.
* Glyph origins against xelatex's PDF: **`tol + rel_tol × size`**, with
  `rel_tol` 0.005 em and size the text rendering matrix's scale (0.06 bp at
  10 bp). This is xdvipdfmx's own measured error (below; lane decision
  2026-10-09). `--rel-tol 0` gives `tol` only. The tight check of the
  positions is the XDV check (0.001 bp).
* Text matrix entries: 1e-4 × size.
* Colours: exact after rounding to 4 decimals.

The report gives the largest glyph deviation among paired glyphs in bp and
in em. Differences are grouped by kind, with the first `--examples`
concrete cases, and `--json` writes everything.

## Results (VERIFIED on mac-m1max-a, 2026-10-09, TeX Live 2026)

* **Self-test** (`run.py --self-test`, formats built fresh): 15 cases
  (l001-l010, d01-d05), 15 equal, **0 structural differences and 0
  differing pixels at 2×**, 17,352 glyphs paired.
  * xelatex against xelatex is byte-identical with the pinned environment,
    so this alone is not a strong check. Each reference PDF is therefore
    also compared with two qpdf rewrites of itself: object streams with
    renumbered, re-compressed objects, and uncompressed QDF. All 30 rewrite
    comparisons give 0 structural differences and 0 px.
  * The XDV check of xelatex's own PDF measures xdvipdfmx and does not gate.
    All 16,978 glyphs that the pages draw themselves were paired, with none
    left over on either side; that includes d03's TikZ glyphs and d05's
    transformed text, so the specials are read as xdvipdfmx reads them.
    12,744 of the paired glyphs are more than 0.001 bp off, max 0.0496 bp
    (0.00498 em), xdvipdfmx's error as `xdvmeasure.py` measures it.
  * One earlier run with 4 jobs failed on a pyobjc lazy-loading race
    (`KeyError: 'CGContextSetShouldSmoothFonts'` in raster.py). The Quartz
    names are now resolved once at import, and the run above passed with
    the fix.
* **Mutation tests** (`test_mutation.py`, 10 tests, OK). The input is a PDF
  made by xelatex, edited in place in qpdf's QDF form with byte counts kept.
  The comparator reports:
  * one `Td` x moved by 0.02 bp: exactly `glyph-position` for that text
    object's glyphs with `--rel-tol 0`; the default tolerance holds it, but
    not a 0.1 bp move;
  * a move of 0.004 bp: no difference, with the deviation measured as
    0.004;
  * a move of 0.5 bp: Core Graphics pixels differ too;
  * `1 0 0 rg` changed to `0 0 1 rg`: exactly `glyph-colour`;
  * one entry removed from `/Annots`: exactly one `link-missing`;
  * an outline title changed: `outline`;
  * the info Title changed: `info`;
  * a named destination's x changed: `dest`;
  * the XDV check: xdvipdfmx's PDF pairs every glyph with the XDV and is off
    by less than 0.02 bp; a text object moved by 0.05 bp shows by that much.
* **Unit tests** (`test_content.py`, 34 tests, OK):
  * the tokenizer: numbers, literal strings with escapes and nesting, hex,
    names with `#xx`, dictionaries, inline images;
  * matrices;
  * text positioning: Tj/TJ, Tc/Tw/Tz/Ts/Tr, T*/'/TD/Tm, cm, q/Q, rotation;
  * paths: re, v/y, clips, closing paints, scaled line width and dashes;
  * colour spaces and ExtGState opacity;
  * a form with an image (nested CTMs) and an inline image;
  * ToUnicode;
  * the matching: rule equivalence, both tolerances, content order, moved,
    swapped, missing/extra, path colour/geometry;
  * structural comparison: numbers as values, the first differing path,
    pattern and function tokens;
  * the XDV specials: bcontent with `cm` and q/Q, btrans rotate, x:scale,
    x:rotate, x:gsave/x:grestore, pdf:literal about the point, btrans
    keywords;
  * the baseline judgement.
* **Outline digests are stable across subsets**: 1,164 distinct
  (font, glyph) pairs in the 14 reference PDFs, 133 of them in more than one
  PDF's subset, and none with two digests. Every glyph has a digest,
  including the Type 1 `FontFile`s of the PDFs l010 includes.

## Visual floor

0 differing pixels is the target only for a writer that places glyphs as
xdvipdfmx does. FlashTeX's writer places them at TeX's exact positions (the
XDV check, 0.001 bp), and xdvipdfmx's own glyphs are up to ±0.005 em away
from those (below). At 2× a shift of a few thousandths of a bp moves a
glyph's anti-aliased edge pixels, so those pixels differ while the glyphs
are the same and correctly placed. The floor is therefore the per-case
pixel count of a run of the writer whose structural report is clean.

**Measured floor** (VERIFIED 2026-10-09, mac-m1max-a). The candidate is the
Unicode-mode PDF writer of #1712 (stacked on #1710), built as
`flashtex-xetex`, compared by this harness at 94318fdab. All 14 cases ran:

* every glyph matched, and the XDV check's largest error was 0.000009 bp;
* 3 structural differences, each only the code point chosen for a
  ToUnicode entry:
  * l004 glyph 30: U+037E vs `;`;
  * l007 glyph 2705: `∣` vs `|`;
  * l008 glyph 1397: `ˆ` vs U+0302;
* 31,597 differing pixels at 2×:

| case | px | case | px |
|---|---|---|---|
| l001 | 505 | l008 | 460 |
| l002 | 590 | l009 | 590 |
| l003 | 794 | l010 | 621 |
| l004 | 952 | d01 | 293 |
| l005 | 20,142 | d02 | 1,050 |
| l006 | 4,270 | d03 | 784 |
| l007 | 502 | d04 | 44 |

**The committed baseline** (`baseline.json`, VERIFIED 2026-10-09,
mac-m1max-a). The candidate is `flashtex-xetex` built in a private target
directory (`cargo build --release -j 4`) from
`agent/mac-claude-a/xetex-s2b-pdf` at
**19c456c0f6aa9b8e25337dca395747b0e8f4bd53** (#1712, on #1710).

* All 15 cases (d05 added): every glyph matched.
* The XDV check now covers glyphs under specials, including d03's 27 TikZ
  glyphs and d05's: all 16,978 glyphs paired, 0 more than 0.001 bp off,
  max 0.000021 bp.
* The same 3 ToUnicode differences.
* 31,838 px at 2×: the table above plus d05's 241.
* `run.py --engine … --baseline baseline.json` then exits 0 with 15 of 15
  cases passing.
* Two runs of that binary gave identical pixel counts.

When the writer changes, rerun with `--write-baseline`, review the change in
the file, and commit it.

**Belief, not measured pixel by pixel:** the floor is xdvipdfmx's own
rounding (3-decimal `Td`, integer TJ kerns, integer `/W`; see the next
section), not an error of the writer. The writer's glyphs are within
0.00001 bp of TeX's positions, and xdvipdfmx's are up to ±0.005 em from
them. **Owner question:** emulate xdvipdfmx's rounding to reach 0 px, or
accept this floor as the gate.

## Measurements: xdvipdfmx's precision (VERIFIED 2026-10-09)

PLAN §3.5 called "positions rounded to 0.01 bp" a belief to measure.
`xdvmeasure.py` makes each corpus PDF in PDF mode. One more `-no-pdf` pass in
the same directory keeps the XDV, and `xdvipdfmx` is run on it. The result is
not byte-identical to the PDF-mode PDF (the ToUnicode streams differ), but it
is structurally equivalent on every case. The script then measures:

**Digits after the decimal point** in the operands of the content streams
(digits:count):

| operator | digits |
|---|---|
| `Td` | 0:367 1:7 2:124 3:1868 (at most 3) |
| `TJ` (kerns) | 0:4337 (always integer thousandths of the font size) |
| `Tf` (size) | 0:105 3:1 4:706 (9.9626) |
| `Tm` | 0:6 1:1 2:1 3:3 5:1 |
| `cm` | 0:2093 1:662 2:25 3:922 4:12 5:36 |
| `m` `l` `c` `re` | up to 5 (TikZ/xcolor/graphicx literals pass through) |
| `w` | 3:3 4:74 5:11 6:1 |

**Glyph origins against TeX's exact positions.** The PDF's glyph origins
(this interpreter) are paired in page order with the XDV's: native
`set_glyphs`, plus TFM characters with DVItype-scaled widths. The XDV is
converted with its num/den/mag and the origin is 1 in, 1 in from the top
left. 16,846 glyphs were paired, with at most 0.05 bp left over for every
pair. That pairing also independently checks the interpreter. Results:

* Overall: **max 0.0496 bp = 0.00498 em** (l004, Times New Roman,
  `FontFile2`).
* CFF OpenType (`CIDFontType0C`, Latin Modern, TeX Gyre, STIX Two,
  Libertinus): max 0.0167 bp = 0.0011 em.
* TFM Type1C: max 0.0057 bp = 0.0006 em.
* The 27 glyphs of d03 sit inside transformations that TikZ's specials set
  up. This measurement walks the XDV in page order without the specials,
  so they are not measured (the XDV check pairs them: max 0.0092 bp).

Why: in l004 xdvipdfmx writes **a whole line in one `TJ`** (not one `BT … Td
… TJ ET` per word), with integer kerns. It also rounds a TrueType font's
`/W` widths to integer thousandths (569/2048 em = 277.83 is written as 278).
xdvipdfmx tracks its position with the exact advance, but a viewer advances
by `/W`, so the drift accumulates along the TJ. Vertical positions are off by
up to 0.0005 bp, which is Td's 3 decimals.

So "0.01 bp" is not xdvipdfmx's precision: its glyphs are up to 0.0011 em
(CFF) and 0.005 em (TrueType) from TeX's positions. The lane decided
(2026-10-09) that FlashTeX's writer does not reproduce that drift and places
glyphs at TeX's exact positions. The comparison with xelatex's PDF
therefore allows `0.01 bp + 0.005 em`, and the XDV check holds the writer
to 0.001 bp of TeX.

**Glyph numbering** (VERIFIED by drawing the same glyphs from the font
files):

* TrueType subsets keep the original numbering: `/CIDToGIDMap /Identity`,
  `maxp.numGlyphs` = the largest glyph used + 1. All 154 Times New Roman
  glyphs of l004 have the outline of the same glyph id in
  `/System/Library/Fonts/Supplemental/Times New Roman.ttf`.
* CFF subsets are CID-keyed with a **compact, non-identity charset** (subset
  index to CID), and the CID is the original glyph id. All 195 LM Roman 10
  glyphs of l002 match `lmroman10-regular.otf` by CID. Decoding a CID
  through the charset would give the subset index, which depends on the
  subset, so the CID is the identity.

## Cases (`cases/`)

Each case has a first-line `% name: what it covers` comment, as the l00x
cases do.

* `d01-hyperref`: link borders, `\ref`/`\pageref`/`\autoref`/`\nameref`,
  `\url` with `?&#`, `\href` (including `mailto:` and a link broken across
  lines), footnote links, `\hyperlink`/`\hypertarget`, `\phantomsection`,
  a three-level outline with an unnumbered entry, docinfo, three pages.
* `d02-xcolor`:
  * colours: rgb, cmyk, gray, RGB 0-255, HTML, named (dvipsnames),
    `\colorlet`, mixes, tints and complements;
  * boxes: `\colorbox` and `\fcolorbox`;
  * rules: coloured, thin and thick;
  * a coloured paragraph (with nested colour) across a page break.
* `d03-tikz`:
  * paths and fills: arrows (including arrows.meta), dashes and dash
    patterns, round caps and joins, Bézier curves, even-odd fill, a
    `double` line;
  * text: nodes, with rounded corners and rotated;
  * transforms: xshift/scale/yslant scopes;
  * effects: opacity (fill and draw), axial and ball (radial) shadings, a
    clip with text, a tiling pattern.
* `d04-hyperref-colorlinks`: colorlinks, `bookmarksopen`/`bookmarksnumbered`,
  Unicode titles in bookmarks and docinfo (UTF-16 strings), `pdfstartview`,
  `pdfpagemode`, roman then arabic page labels, a citation.
* `d05-transformed-text`: text under graphicx's `\rotatebox` (`pdf:btrans
  rotate`, about the origin and the centre), `\scalebox`, `\reflectbox`
  and `\resizebox` (`pdf:btrans` and `x:scale`), nested; TikZ nodes that
  are rotated, slanted and scaled (`cm` in `pdf:code` within
  `pdf:bcontent`). It is there for the XDV check.

Images come only from TeX Live's mwe package (l010).
`tools/xetex-lockstep/pictures` holds header-only files and cannot be
rendered.

## Known gaps (beliefs, not measured)

* The XDV check follows the writer's reading of the specials, not
  xdvipdfmx's source. The self-test pairs every one of xdvipdfmx's glyphs
  that way, but only for the specials of this corpus. Forms (`pdf:bxobj`)
  are not interpreted: a glyph drawn inside one is not checked against the
  XDV.
* The XDV comes from the reference engine. A candidate whose typesetting
  differs (its own XDV, P-T1) is caught by tools/xetex-lockstep, not here.

* Non-link annotations and sampled-function and ICC data are compared
  exactly (by canonical value or digest), not within a tolerance.
* Glyph order is compared only among paired glyphs. The painting order of
  paths, images and forms is not compared; the visual comparison catches
  visible order changes.
* Optional content, structure trees and tags, XMP metadata, `/ToUnicode`
  for unused codes, `/ActualText`, font descriptors' metrics (`/Ascent`,
  `/Flags`, ...), and Type 3 fonts beyond `/FontMatrix` are not compared.
* Vertical writing (`Identity-V`) is read as horizontal. Predefined CMaps
  other than Identity are read as Identity, with a warning.
* `--glyph-identity outline` needs embedded programs that fontTools can
  draw. A glyph without a program falls back to its id.
* Core Graphics does not draw link borders (no appearance streams), so
  links are only compared structurally.
