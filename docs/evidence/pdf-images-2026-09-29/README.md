# PDF backend: image and PDF inclusion ported, libpng and xpdf linked (2026-09-29)

Lane **P3-IMAGES** (kabir-claude, mac-m5pro-kabir), DESIGN.md §6.3, §1.1 P-T2,
§4.1 "C parts", §12 P3. Branch `agent/kabir-claude/p3-images`, from
`agent/kabir-claude/p3-fonts` (PR #1197), whose README
(`docs/evidence/pdf-backend-2026-09-29/`) this continues.

The engine (`crates/flashtex-engine`, GPL-2.0-or-later) now includes PNG,
JPEG, JBIG2 and PDF files with `\pdfximage`, as pdfTeX 1.40.29 does. Before
this lane `read_image` stopped the run ("image inclusion is not implemented
yet"), which kept the 7 beamer fixtures out of P-T2.

## Source of record

pdfTeX 1.40.29's C sources from the TeX Live source tree, the same pin as
`third_party/pdftex` (tag `texlive-2026.1`, commit
`6a300188053b8f2ded89dbd52293732a706b9c0e`), path `texk/web2c/pdftexdir/`.
They were read, not vendored; each Rust function names its C original.

| file | sha256 |
|---|---|
| `writeimg.c` | `b3f023fd322e35fb4d3e44b095d1a8c9dccea1ca3a5d4ce3706757e36841e820` |
| `writepng.c` | `4e3d5ae08e545c6b8bf84d61c0b0abc13fd8807e80f46536bd2b0d6dd0e8861a` |
| `writejpg.c` | `adbf73948366d4c8e2e91d6d020d450e7e275b0b15d9266f9ea505adac0e7f55` |
| `writejbig2.c` | `aa54c294da020a7b07193d4dc0373375d736b62ca65eb75170af9e0696f940ad` |
| `pdftoepdf.cc` | `f5f88cfbaa8ac00a505a1421d538350f62b4857c5842ac38351dd22a23aaee25` |
| `epdf.c` | `8a6da96cbf099905a1a1f58c1c11e82a298a0ca97718e6c6cf4e3e7e93eb03d0` |
| `image.h` | `86f15c9491db0a919e922aac1d8531f86616de4e16054ff216e0c0c2100f9fdb` |
| `pdftex-common.h` | `8747d30ef6a1deb227b81ecee113d4ef1bd202d30aa1d2e39bf7174fff323f60` |

The libraries are vendored byte-for-byte from the same commit:
`third_party/libpng` (libpng 1.6.55, `libs/libpng/libpng-src`) and
`third_party/xpdf` (xpdf 4.06, `libs/xpdf/xpdf-src`), each with a README
(pin, file list, build, licence) and `SHA256SUMS`; every vendored file was
checked byte-identical to the TeX Live checkout. TeX Live 2026's
`pdftex --version` reports exactly these: "Compiled with libpng 1.6.55;
using libpng 1.6.55 ... Compiled with xpdf version 4.06".

## Reuse before building (DESIGN.md §1): per file, port or link

The rule applied as in the font lane: pdfTeX's own `pdftexdir/*` files read
and write the engine's globals (`pdfbuf`, `pdfptr`, `pdflastbyte`,
`objptr`, `pdfpagegroupval`, `fixedpdfminorversion`, ...) and call its
routines (`pdfbegindict`, `pdfnewobjnum`, `pdfendstream`, the font
backend's `lookup_fontmap` and descriptor tree), which are fields and
methods of the Rust `Globals`; linking them would need a C ABI shim over all
of that, so they are **ported**. The libraries they call have no such
coupling and are **linked unmodified**: that gives pdfTeX's behaviour by
construction, where a rewrite would have to reproduce it.

| C file / library | decision | Rust module | deciding evidence |
|---|---|---|---|
| `writeimg.c` | port | `src/pdftex/images.rs` | engine globals; 1:1 dispatch, type detection, image table, (un)dump |
| `writepng.c` | port | `writepng.rs` | writes `pdfbuf` directly (the IDAT copy and the row loops); the IDAT copy-through ("PNG copy") is ported as is (DESIGN §6.3) |
| **libpng 1.6.55** | **link** (`third_party/libpng`, `csrc/png_shim.c`) | `writepng.rs` | pdfTeX's image bytes are libpng's output: unfiltering, Adam7, `png_set_tRNS_to_alpha`, `strip_alpha`, `strip_16`, `png_set_gamma` tables, and which malformed files it accepts. Measured: 16 PNG test images x 6 settings byte-identical, and 336 malformed PNG mutants with the same outcome as pdftex (below); speed equal to pdftex (same library, below). A Rust decoder (`png`, `zune-png`) would have to reproduce libpng's transforms and error decisions exactly to meet the bar, and would not be faster than libpng with NEON on the paths that matter |
| `writejpg.c` | port | `writejpg.rs` | reads markers and copies the file; no library involved |
| `writejbig2.c` | port | `writejbig2.rs` | reads segment headers and copies; no library involved |
| `pdftoepdf.cc` | port | `pdftoepdf.rs` | writes pdfTeX's output through the engine's globals (`pdf_puts`, `pdfout`, `pdfbeginobj`, the font backend) |
| **xpdf 4.06** (C++) | **link** (`third_party/xpdf`, `csrc/xpdf_shim.cc`) | `xpdf.rs` | see the next section |
| `epdf.c` | port | `epdf.rs` | the font backend's descriptor tree (`register_fd_entry`, `lookup_fd_entry`, P3-FONTS) |
| C `FILE` (`getc`, `fseek`, `fread`, `feof`) | reimplemented | `cfile.rs` | a model of the C semantics the readers depend on at and past the end of a file; files are read whole |
| C `sprintf("%.1f"/"%.8f")` | link (the platform's) | `cfmt::fmt_f` | the `/BBox`, `/Matrix` and version-warning numbers, as the fonts lane does for `%g` |

### xpdf: link it, or reproduce it in Rust?

pdftoepdf's output depends on xpdf's parse at every step: which objects the
xref (or its reconstruction, for a damaged file) finds, object streams,
page-attribute inheritance (`/Resources`, boxes, `/Rotate`), box clipping,
named destinations, how the lexer reads each real number (then written back
with six decimals), stream `/Length` handling, filter decoding for content
arrays, and, for font replacement, the encoding `GfxFont::makeFont` derives
from the font dictionary and the embedded Type 1 / CFF program (FoFi). That
is about 33,000 lines of xpdf (Lexer, Parser, XRef, Object, Array, Dict,
Stream, Catalog, Page, Link, PDFDoc, GfxFont, FoFi*, Decrypt, font tables)
that a Rust parser would have to match decision for decision. No Rust crate
does (`lopdf` and `pdf` make their own repair, lexing and encoding choices,
and none derives xpdf's glyph names).

| criterion | linked xpdf (chosen) | a Rust PDF parser |
|---|---|---|
| identical output on the fixtures | 82/82 P-T2; the 7 beamer fixtures byte-identical to pdflatex; 16 test documents byte-identical, incl. a damaged xref, object streams, Type 1 and Type 1C font replacement | would need xpdf's behaviour re-derived and proved on every item above |
| malformed PDFs | 252 mutants of 3 PDFs: no crash, no hang, the same exit status, log and PDF as pdftex (below). Every xpdf accessor call is type-checked in the shim, so no malformed object can read a wrong union member | its own failure modes, differing from pdfTeX's |
| build, macOS and Linux | 56 C++ files + a 390-line shim, compiled by `cc` (6.3 s wall, one file at a time, -O3, on the M5 Pro); TeX Live builds the same files with gcc on Linux | none |
| speed | faster than pdftex on the measured case (below) | unknown |

## Verified results (measured on mac-m5pro-kabir, 2026-09-29)

Oracle: TeX Live 2026 `pdftex` 1.40.29 (`/Library/TeX/texbin/pdftex`),
`SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1`. Engine: this branch's
`flashtex-initex` (release). qpdf 12.4.2.

### P-T2 on the parity fixtures

`python3 tools/parity/parity.py --tier fixtures --engine target/p3work/engine.sh --pt pt2 --raster none`
(the test wrapper of the font lane, which maps pdfTeX's
`-interaction`/`-jobname` onto `flashtex-initex` until the CLI lane adds
them; format built by the engine from TeX Live's `pdflatex.ini`):

**P-T2 82/82 fixtures** (was 75/82). Scoreboard: `parity-fixtures-pt2.md`
and `.json` beside this file.

Fixtures that include images: the 7 beamer fixtures (`beamer-blocks-columns`,
`beamer-default`, `beamer-fragile`, `beamer-madrid`, `beamer-overlays`,
`beamer-polish`, `beamer-visuals`), whose navigation symbols are
`beamericon*.pdf` (PDF inclusion) and of which `beamer-blocks-columns` also
includes `figure.png` (PNG, decoded path: it has colour-management chunks).
`thesis-chapter`, `hyperref-toc`, `lab-report` and `conf-paper` use
`\includegraphics[draft]` (no file is read) and `article-twocolumn` names a
figure that does not exist (pdflatex reports the missing file too); they
passed before and still do. All 7 image fixtures pass P-T2, and beyond it:
with `\pdfsuppressptexinfo=-1`, three passes each, **all 7 beamer PDFs are
byte-identical to pdflatex's** (40,494 to 110,489 bytes).

### Byte-identity tests

`crates/flashtex-engine/tests/pdf_images.rs` (runs in `cargo test` where
TeX Live is installed; plain TeX, PDF mode; inputs made by
`tests/images/generate.py`, which regenerates every file byte-identically):

* `images_match_tex_live`: 16 documents, **16/16 byte-identical PDFs and
  identical logs** (program name normalised):
  * PNG, 16 images: RGB/grey/palette/grey+alpha/RGBA, 1, 4, 8 and 16 bits,
    palette with tRNS, grey with tRNS, Adam7 (8 and 16 bits), gAMA 1.0
    (copied) and 0.45455, sRGB, pHYs, IDAT split in three, all five
    scanline filters; each at PDF 1.7 with and without `\pdfimagehicolor`,
    PDF 1.4 (16 bits stripped) and 1.3 (alpha stripped, no group),
    `\pdfimageapplygamma`, uncompressed streams, `colorspace` objects. The
    log shows which went through "PNG copy" (the IDAT copy) and which were
    decoded, as pdftex's does.
  * JPEG: baseline grey, RGB (JFIF per cm), CMYK (Adobe APP14, `/Decode`),
    progressive, Exif resolution.
  * JBIG2: sequential and random-access files, two pages each, the
    `/JBIG2Globals` object of their page-0 segments.
  * PDF: all five page boxes, `/Rotate` 90 and 270, a named destination,
    content arrays (decoded and recompressed), an indirect `/Length`,
    inherited resources, `/Group` (as a page group object and, for the
    second PDF on a page, copied with pdfTeX's warning), `/Metadata`,
    `/PieceInfo`, strings, names and reals of every form pdftoepdf writes,
    a non-embedded and an inline font; font replacement through the font
    map for Type 1 subsets (with `/CharSet`), whole Type 1 fonts (no
    `/CharSet`) and **Type 1C** (`/FontFile3`) fonts, sharing descriptors
    with the host document's own fonts; `\pdfinclusioncopyfonts=1`;
    `\pdfsuppressptexinfo=1` (`/PTEX.FileName`, `/PTEX.PageNumber`,
    `/PTEX.InfoDict`) and `\pdfptexuseunderscore`; object streams; a
    damaged xref (xpdf reconstructs it); the version warning; a PDF
    included twice and on later pages (objects shared, the document freed
    and reopened exactly when pdfTeX does).
* `image_errors_match_tex_live`: 8 fatal errors (progressive JPEG at PDF
  1.2, JBIG2 below PDF 1.4, missing JBIG2 page, missing PDF page, unknown
  destination, PDF version above `\pdfinclusionerrorlevel`, missing file,
  unknown type): **8/8 the same message, no PDF from either**.
* `dumped_images_match_tex_live`: images read by INITEX, dumped with the
  format (`dumpimagemeta`), read again when the format is loaded
  (`undumpimagemeta`), used from saved boxes: **byte-identical**.

Existing gates stay green: `cargo test -p flashtex-engine` (incl.
`pdf_backend`, `latex_format`, `plain_format`), `scripts/flashtex-trip.sh`,
`scripts/flashtex-etrip.sh` (the scratch package has no build.rs; libpng and
xpdf are then left out, as zlib is, and `\pdfximage` stops the run),
`scripts/gate.sh pr`.

### Malformed input (DESIGN §4.5)

`fuzz.py` beside this file (seeded): 12 test inputs (4 PNG, 3 JPEG, 2
JBIG2, 3 PDF), each truncated at 1/5..4/5, with 1-6 random bytes changed
(60 per file) and with runs of up to 64 zero bytes (20 per file): **1,008
mutants. The engine: 0 crashes, 0 hangs, 0 panics.** Against pdftex: the
same exit status on 1,007, the same log on 1,006, and wherever both wrote a
PDF, byte-identical PDFs on 342 of 344. The two differences are JPEGs whose
Exif block points outside itself, where pdftex reads past its buffer
(undefined behaviour, so its result depends on the heap): on one **pdftex
crashes (SIGSEGV)**, on the other it reads a garbage resolution and warns
"too small image resolution ignored". The engine reads those bytes as 0
and keeps the default resolution. (An earlier run of the same mutants saw
only the crash: pdftex's result on the second varies from run to run.)
Every PNG, JBIG2 and PDF mutant behaved exactly as with pdftex.

### Speed

`perf.py` beside this file: medians of 5 runs, seconds, plain TeX, the same
machine (other agents running, so a few ms of noise).

| case | engine (s) | pdftex (s) | engine - empty | pdftex - empty | PDF bytes | PDFs identical |
|---|---|---|---|---|---|---|
| empty document | 0.109 | 0.156 | 0.000 | 0.000 | 8,562 | yes |
| PNG 4000x3000 RGB, IDAT copied | 0.125 | 0.164 | 0.016 | 0.007 | 23,057,350 | yes |
| PNG 4000x3000 RGBA, decoded + `/SMask` | 0.960 | 0.998 | 0.850 | 0.842 | 31,376,600 | yes |
| JPEG 4000x3000 | 0.115 | 0.199 | 0.006 | 0.043 | 4,641,453 | yes |
| PDF, all 236 pages of beamer's user guide | 0.210 | 0.281 | 0.100 | 0.125 | 2,132,558 | yes |

Every whole run is faster than pdftex's (the format loads faster). The
image work alone (minus the empty run) is lower for JPEG and PDF inclusion
and within the noise of these runs (±10 ms, other agents were running) for
the two PNG paths, which are libpng and TeX Live's zlib in both. The PNG paths moved image bytes through
the engine's PDF buffer one call per byte at first (+28 ms on the copy,
+50 ms decoded); they now move slices (`pdf_buf_store`, and `write_pdf`/
`write_zip` in output.rs read the buffer a slice at a time), with `pdfroom`
at exactly the bytes where C calls it.

## Remaining differences, by cause, and owners

1. **Where pdfTeX has undefined behaviour, the engine stops or continues
   deterministically** instead (DESIGN §4.5), each marked in the code: a
   named destination given by page number (pdfTeX reads an unset
   reference; here "destination is not a page"), a missing `/StemV` (read
   as 0), a page object or content-array entry of the wrong type, a
   content stream without `/Length` (a failed `assert` in pdfTeX), a JBIG2
   page-0 reference cycle (unbounded recursion in C; stops after depth
   1,000), a libpng error after `read_png_info` (C longjmps into a returned
   frame; here "libpng: internal error"), Exif and JBIG2 reads past their
   data (read as 0). Owner: this lane; none occurs in a well-formed file.
2. **INITEX by default.** Without `-fmt`, `flashtex-initex` runs as INITEX
   (`main.rs`), where pdfTeX deliberately never frees an image
   (`deleteimage` returns early, since images can be dumped): an included
   PDF then stays open and a later `\pdfximage` of it reuses the objects
   already copied. That is pdfTeX's INITEX behaviour too; production runs
   (`-fmt=...`, as the parity wrapper and the tests use) free and reopen as
   `pdftex &fmt` does. Owner: P2-PT1-FIXTURES (the command line).
3. **Program name in warnings** (`kpse_invocation_name`), as in the font
   lane. Owner: `system.rs` (P2-PT1-FIXTURES).
4. **`-recorder`**: `readimage` calls `recorder_record_input`; the engine has
   no recorder yet. Owner: system/CLI lane.
5. **xpdf's configuration file.** Like pdfTeX, xpdf's `GlobalParams` reads
   `~/.xpdfrc` or the system `xpdfrc` if one exists; that is a read outside
   the project and the TeX trees (DESIGN §4.5). None exists on the
   machines measured. Owner: the sandbox/determinism work (a
   `GlobalParams` built with a config path that exists and is empty would
   close it without changing any other behaviour).
6. **Licence note for §3's legal review.** xpdf is GPL v2 or v3, explicitly
   not "or later"; an engine binary linking it is distributable under GPL
   v2 or v3 (pdfTeX is in the same position). Owner: the owner's §3 review.
7. **Linux build** is expected to work (TeX Live builds the same sources
   with gcc; the `cc` crate links `libstdc++`) but was not built locally
   (no Linux VM running); CI's Ubuntu job is the check.

## Reproduce

```sh
CARGO_BUILD_JOBS=4 cargo build --release -p flashtex-engine
cargo test --release -p flashtex-engine --test pdf_images     # needs TeX Live
python3 crates/flashtex-engine/tests/images/generate.py       # regenerates the inputs
python3 docs/evidence/pdf-images-2026-09-29/fuzz.py 60        # malformed inputs
python3 docs/evidence/pdf-images-2026-09-29/perf.py           # speed (needs Pillow)
# P-T2: build pdflatex.fmt with the engine, then run tools/parity with an
# --engine that accepts pdfTeX's command line (see the font lane's README).
```
