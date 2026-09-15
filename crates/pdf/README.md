> Experimental runtime-v1 export component. Structural validity and narrow native
> font tests do not establish exact LaTeX byte or pixel identity. Runtime-v1 lacks
> authoritative font IDs/original GIDs/typed rules, so fallback fonts and rule
> conventions remain fidelity blockers; rendering-v2 negotiation is separate.
> The separate exact route (`flashtex_pdf::exact`, below) takes glyph ids,
> verbatim decimals and typed operators from a producer that has them.

# flashtex-pdf

Original Rust PDF writer for FlashTeX (task FT-009). Takes runtime-v1
`compile_result` pages (`docs/contracts/runtime-v1.md`) and writes a PDF 1.4
document by hand: catalog, page tree, page objects with `MediaBox`, content
streams, cross-reference table, trailer. No TeX engine is involved anywhere,
and the crate has zero external dependencies (the JSON reader is in-tree, like
`crates/compiler`), so it builds offline and deterministically.

```sh
cd crates/pdf
cargo test
cargo run --bin flashtex-pdf -- --out out.pdf < ../../protocol/fixtures/compile-result.json
cargo run --bin flashtex-pdf -- result.json --out out.pdf --verify
# Opt-in: embed a subset of a Unicode TrueType font for characters outside
# WinAnsi and Symbol (see "Font embedding" below before redistributing output).
cargo run --bin flashtex-pdf -- result.json --out out.pdf --embed-font /path/to/font.ttf
FLASHTEX_UNICODE_FONT=/path/to/font.ttf cargo run --bin flashtex-pdf -- result.json --out out.pdf
open out.pdf
```

## What is implemented

- Library: `flashtex_pdf::render_pdf(&CompileResult) -> Result<PdfOutput, PdfError>`
  and `render_envelope(&str)` for a raw runtime-v1 envelope. `PdfOutput` carries
  `bytes` and `warnings`; a non-empty warning list means the document was
  approximated somewhere and the caller should surface it, not hide it.
- CLI `flashtex-pdf [INPUT.json] --out OUTPUT.pdf [--verify]`. Reads the envelope
  from a file or stdin, prints warnings to stderr, exits 1 for an unsupported
  `protocol_version`/`type` or unrenderable pages, 2 for usage errors.
  `--verify` re-reads the produced bytes and checks the xref table before writing.
- One page object per runtime-v1 page with `MediaBox [0 0 width_pt height_pt]`.
  Any page size is accepted; nothing assumes US Letter.
- Every `kind: text` item becomes one `BT x y Td … ET` block with
  `y = height_pt - baseline_y_pt`, i.e. the runtime-v1 top-left origin converted
  to PDF's bottom-left origin. Coordinates are written to a thousandth of a point.
  Inside the block the item is split into font runs, each `/Fn size Tf (bytes) Tj`:
  `/F1` Times-Roman (WinAnsiEncoding) for everything WinAnsi covers, `/F2` the
  base-14 `Symbol` font (its built-in encoding) for Greek letters and the
  mathematical operators listed in `src/encoding.rs`. Because the runs share one
  text object, the viewer advances between them with the real base-14 widths;
  this crate ships no width tables.
- **Negotiated layout capabilities**
  (`docs/contracts/runtime-v1-layout-capabilities.md`). `payload.layout_capabilities`
  on the `compile_result` is the accepted set (≤16 unique strings, each ≤64
  bytes). Its presence switches the writer to the *negotiated route*:
  - `rules-v1`: items `{"kind":"rule","x_pt","y_pt","width_pt","height_pt","source"}`
    give the rectangle's **top-left** corner in page space (y downward) and
    positive, finite dimensions (magnitudes ≤ 1,000,000). They are drawn as
    `x (height_pt - y_pt - height_pt) width_pt height_pt re f`, opaque black,
    in item order (paint order). A `rule` item when `rules-v1` is not in the
    accepted set is an error naming its source range; so is a `rule` on the
    legacy route.
  - `font-hints-v1`: text items may carry `font:{family,weight,style}`
    (see "Font hints" below). A hint when `font-hints-v1` is not accepted is
    an error.
  - Any other item kind under negotiation is an error naming the kind and its
    `path:start-end` source range. Nothing is skipped silently.
  - On the negotiated route U+2500 is ordinary text (it is not in any of the
    fonts here, so it becomes `?` with a warning); typed rules replace it.
- **Legacy route** (no `layout_capabilities`): only `text` items are defined.
  Unknown kinds are skipped with a warning naming them (documented legacy
  behaviour); `rule` is still an error. **Legacy fraction rules:** the FT-002
  compiler emits a fraction bar as a text item consisting only of U+2500 (`─`)
  repeated N times, each assumed 0.5 em wide, with the item's baseline at the
  bar's bottom edge and thickness 0.06 em of the parent size (the item itself
  is set at 0.7 of the parent). Such items are rendered as filled rectangles
  of width `N × 0.5 × font_size_pt` and thickness `0.06/0.7 × font_size_pt`.
  **This is an approximation:** width and thickness are inferred from the font
  size, not measured, and it exists only on the legacy route. `rules-v1`
  carries the real geometry.
- **Font hints (`font-hints-v1`).** Each hinted text item is resolved to a face:
  - `Latin Modern*` / `LMRoman*` → `lmroman10-{regular,bold,italic,bolditalic}.otf`
    from the Latin Modern directory (the embedded document font's directory
    when it is LM, else the first `auto` candidate that exists,
    `FLASHTEX_LM_DIR` first). Each used face is embedded as **its own** whole
    CFF font object (`/F4`, `/F5`, …), with its own `/W` and ToUnicode; a
    document using regular, bold, and italic therefore carries three CFF
    tables (about 250 KB with all four LM Roman faces).
  - `Times*` → base-14 `Times-Roman` (`/F1`), `Times-Bold`, `Times-Italic`,
    `Times-BoldItalic` (extra `/Fn` objects, not embedded).
  - Anything else → substituted by the document face at the requested
    weight/style (Latin Modern when LM is the document face, else the Times
    variant) and reported once per distinct hint:
    `font "Palatino" (italic) substituted by 'Latin Modern Roman italic';
    requested metrics were not preserved`. A Latin Modern hint with no LM
    installation is substituted by the Times variant, also with a warning.
  - Fallback inside a hinted run is the same as for the document face:
    embedded face → Symbol → Times → `?`, or Times variant → Symbol →
    document embedded font → `?`.
  - Hints only fix style intent; they carry no font bytes, GIDs, or advances,
    so this is not byte/pixel parity (the contract says the same).
- `(`, `)`, and `\` are escaped in literal strings.
- **Font embedding (opt-in).** `RenderOptions { embed_font }`, `--embed-font
  PATH|auto`, or `FLASHTEX_UNICODE_FONT` embed a subset of a Unicode TrueType
  font as `/F3` for characters neither base-14 font has. See below.
- Structural self-check (`flashtex_pdf::verify`) that parses the header, `startxref`,
  every xref entry, and confirms each offset lands on `N 0 obj`, plus a
  content-stream reader that recovers `Tf`/`Td`/`Tj` runs and `re f` rules for tests.

## Exact export route (`flashtex_pdf::exact`, issue #25)

The runtime-v1 route above selects fonts by character and formats numbers
to three decimals. The exact route is the additive API GitHub issue #25
asks for and does neither. It is a second entry point into the same
container writer, not a second PDF implementation:

- **Numbers are verbatim.** Every operand is a `Decimal`: a validated PDF
  numeric token (`-?digits(.digits)?`, no exponent, at most 64 characters)
  carried as text. `11.9552`, `708.045`, `0.398`, `216.00000000001` are
  written exactly as given; nothing goes through `f64`.
- **Text is shown by code, not character.** `ExactFont::CidCff` and
  `ExactFont::CidTrueType` are `Type0`/`Identity-H` fonts whose two-byte
  codes *are* the source font's glyph ids: the CFF program is rewritten by
  `crate::cff` as a CID-keyed CFF whose charset maps CID = original GID
  (charstrings, global and local subroutines copied byte for byte; one FD
  with the original Private DICT), and the TrueType program keeps its glyph
  order (`TrueTypeFont::subset_keep_gids`, `/CIDToGIDMap /Identity`).
  `ExactFont::Simple` is a one-byte-code font with an explicit
  `/Differences` encoding and a Type 1 (`FontFile` with `Length1/2/3`),
  `Type1C`, or TrueType program, or a standard-14 name, the form pdfTeX
  writes. `GlyphRun { font, size, glyphs: [PlacedGlyph { gid, origin }] }`
  expands to `BT /F size Tf 1 0 0 1 x y Tm (codes) Tj … ET`; a glyph without
  an origin continues the previous string at the font's advance.
- **Typed operators.** `Op` is the bounded set `q Q cm w J j d g G rg RG m l
  c h re S f f* n W W* BT ET Tf Td Tm Tj TJ`; `Op::rule(x, y, w, h)` is
  `re f`. A page's `Content::Ops` is serialised one operator per line;
  `Content::Verbatim(bytes)` is parsed into the same set for validation and
  then inserted unchanged. Anything else (`gs`, `sc`, shading, images,
  inline images, exponents, nested arrays) is an error naming the page and
  operator index; nothing is dropped or rounded.
- **Marked ActualText.** Exact producers may put `Op::BeginActualText(text)` and
  `Op::EndMarkedContent` around consecutive glyph operators. The writer emits
  `/Span <</ActualText <FEFF…>>> BDC` and `EMC`, encoding `text` as UTF-16BE
  hex with a BOM. The pair may enclose font switches and one or more `BT … ET`
  blocks. It does not change any font's `ToUnicode` map.
- **Validation before writing.** Balanced `q`/`Q` and `BT`/`ET`, text
  operators only inside a text object, path segments only after a current
  point, painting only with a path, every `Tf` naming a declared font that
  the page's `/Resources` lists, every one-byte code inside
  `FirstChar..=LastChar`, every two-byte code inside the declared glyph set
  (`CidFont::glyphs`; empty means unconstrained for programs carried over
  from another producer), even byte counts for two-byte fonts, positive
  page sizes, bounded sizes (content 64 MiB, 4 M operators per page, font
  programs 64 MiB).
- **Deterministic.** No `/ID`, no dates, fixed object order (catalog, pages,
  info, page/content pairs, fonts by resource name). The same
  `ExactDocument` gives the same bytes twice (tested), and re-emitting the
  writer's own output through the reader is a fixed point.
- **White pages.** No background is painted; PDF user space is bottom-left,
  y up, and the caller does any flip (rendering-core's adapter already
  does), so no arithmetic happens in this crate.

```rust
use flashtex_pdf::exact::*;
let font = TrueTypeFont::load(Path::new("lmroman10-regular.otf"))?;
let (f1, outcome, note) = ExactFont::cid_from_opentype(&font, &gids, to_unicode)?;
// outcome: CffSubset (GIDs kept as CIDs) | CffWhole (seac/CID-keyed source, reported) | TrueTypeIdentity
let run = GlyphRun { font: "F1".into(), size: Decimal::new("12")?, glyphs };
let mut ops = run.to_ops()?;
ops.extend(Op::rule(Decimal::new("72")?, Decimal::new("690.25")?, Decimal::new("28.5")?, Decimal::new("0.398")?));
let page = ExactPage { width: Decimal::new("612")?, height: Decimal::new("792")?, content: Content::Ops(ops), fonts: None };
let doc = ExactDocument { pages: vec![page], fonts: BTreeMap::from([("F1".to_string(), f1)]), images: BTreeMap::new() };
let pdf = render_exact(&doc)?;   // PdfOutput { bytes, warnings: [] }
```

**Bounded subset identity.** `CffFont::subset` refuses, with an explicit
error rather than a guess, CID-keyed sources, Type 1 charstrings, and any
retained glyph that composes an accent through `endchar` (`seac`), because
a CID-keyed program would resolve the components by CID instead of name;
`cid_from_opentype` then embeds the whole `CFF ` table (glyph ids are still
the font's own) and says so in its note. Latin Modern's 821 glyphs contain
no `seac`; a nine-glyph subset is 22,557 bytes against the 61,140-byte whole
table. The subset tag in `/BaseFont` is derived from the glyph set and the
source program's SHA-256, so the same request always names the same font. Type 1 programs
(`crate::type1`, `ExactFont::type1_subset`) are subset the way pdfTeX does
it: retained charstrings and needed subroutines byte-identical, unused
subroutines blanked, deterministic eexec, `Length3 0`; the classifier
compares two embedded Type 1 programs charstring by charstring. Subsets keep only the
subroutines the retained glyphs reach (renumbered, call operands rewritten)
and only the Top DICT's strings; `CffFont::expanded_charstring` (subroutines
inlined) is byte-identical before and after, which is what the identity
tests compare. Latin Modern Math: 7 glyphs in 1,753 bytes.

**Reading references and classifying differences.** `crate::reader` reads
a finished PDF (this crate's or pdfTeX's: xref streams, object streams,
`FlateDecode` via the in-tree `crate::inflate`) into an object model with
verbatim numbers; `crate::compare::reemit` rebuilds it as an
`ExactDocument` and `crate::compare::classify` tags every difference
between two files as `PageGeometry`, `ContentFormatting`,
`ContentOperands`, `ContentOperators`, `ContentUnsupported`,
`FontProgram`, `FontMetadata`, `FontResources`, `ObjectLayout`,
`Compression`, or `DocumentIdentity`. The `flashtex-pdf-exact` binary
exposes `reemit`, `classify` (exit 0 only when content operators and font
programs are identical on every page) and `dump`. Against MacTeX 2026
pdflatex and xelatex output for all 18 visual-corpus fixtures under the
harness's four preambles, the re-emitted files have byte-identical content
streams and font programs and pixel-identical CoreGraphics rasters; the
remaining differences are exactly object layout, stream compression and
document identity (`/ID`, dates, producer). Details, the reference profile
and the per-fixture table: `docs/exact-export-classification.md`. This
establishes what the container preserves, not what FlashTeX's compiler
emits.

**Feeding it from rendering-v2.** `flashtex-pdf-exact from-v2 LIST.json --out
OUT.pdf [--font-dir DIR]` (`crate::v2`) consumes the `display_list` envelope
of `flashtex-render --v2`: ticks (`bp_2pow20`) become exact decimals after an
integer y flip, every glyph is placed by original GID at its absolute origin
(continuing the previous `TJ` segment when the gap from the natural advance
is an exactly representable thousandth of the size — zero joins the string,
non-zero is a `TJ` kern — and starting its own `Tm` otherwise; the written
operators replay to the envelope origins exactly, `exact::glyph_positions`),
rules become `re f`, fonts
are resolved by content hash from `--font-dir`/`FLASHTEX_FONT_DIRS`/
`FLASHTEX_LM_DIR`/the TeX Live Latin Modern directories and embedded as
GID-preserving subsets, cluster text becomes ToUnicode. An optional
`"actual_text":"⟹"` field on consecutive `glyph_run` items coalesces runs
with the same value into one `/ActualText` marked-content span, including
across font switches and `BT … ET` blocks; absent fields keep the old
serialization and ToUnicode behavior. `opentype-cff` and
`static-truetype` are accepted; `core14-afm`, alpha, non-integer
ticks and non-terminating colours are errors. Both SHA-256(bytes) and
font-engine's SHA-256(bytes ‖ face index) are accepted as `sha256` (the
latter is reported as a deviation). The measured gap between
`flashtex-render --v2 → from-v2` and pdflatex-lmodern on the 18 corpus
fixtures is in `docs/v2-adapter-gap.md`: text-only fixtures differ at the
anti-aliasing level only (0 ink-only pixels on 01/04/05/17), math and lists
differ where the pipeline's own diagnostics say they do. This glyph-run route
(embedded fonts, searchable text) is complementary to rendering-core's
outline route (`pdf_export.rs`, paths only).

**Images (`display-list-v2-images`, `protocol/proposals/display-list-v2-image.md`
§5.4).** `from-v2 … --project-root DIR` (`v2::from_v2_rooted`) exports
`image` items; without a root they are refused. Each file is read under the
root with every component checked to be a real directory/file (no symbolic
links, no `.`/`..`/absolute paths, device+inode re-checked after open), and
its length and SHA-256 must equal the item's before decoding; a pixel size,
`pdf_box` or `pdf_rotate` that disagrees with the bytes is refused as stale.
What is written follows pdfTeX 1.40.29 (TeX Live 2026), measured:

- PNG (`crate::raster`, zero-dependency): decoded (IDAT inflate, all five
  filters, Adam7) and re-encoded `/FlateDecode` without a predictor, as
  pdfTeX does. Gray/RGB keep depth 1/2/4/8/16; palettes become
  `[/Indexed /DeviceRGB hival lookup-stream]`; gray+alpha/RGBA split into
  colour plus an 8-bit `/SMask` (16-bit alpha keeps its high byte) and the
  page gets `/Group << /S /Transparency /CS /DeviceRGB /I true >>`; palette +
  `tRNS` becomes RGB8 + `/SMask` without a page group. `gAMA`/`iCCP` are
  ignored (`\pdfimageapplygamma=0`). A `tRNS` colour key on gray/RGB is
  refused. The compressor (`crate::deflate`, fixed Huffman + LZ77) is not
  zlib-identical; the decoded samples are (checked by hash).
- JPEG: bytes unchanged as `/DCTDecode`; size/components from the SOF
  (baseline, extended, progressive); Adobe APP14 CMYK gets
  `/Decode [1 0 1 0 1 0 1 0]`. 12-bit, lossless, arithmetic and CMYK without
  APP14 are refused.
- PDF page (`crate::images::from_pdf_page`): a Form XObject with `/BBox` =
  CropBox ∩ MediaBox (graphicx default `pagebox=cropbox`, `pdftex.def`),
  `/Rotate` as `/Matrix` (90: `[0 -1 1 0 -lly urx]`), the content stream
  copied raw with its filter (arrays joined and re-compressed), `/Resources`
  and `/Group` deep-copied with renumbering (object streams read; links back
  into the page tree become `null`; encrypted files refused). pdfTeX's
  `/PTEX.*` keys are not written (`/PTEX.FileName` is an absolute path).
- Placement: `q [a -b c -d e H-f] cm … /ImN Do Q` from the item's transform
  (its own decimals; `H - f` exact from ticks); forms add
  `1/W 0 0 1/H 0 0 cm` (12 decimals) and, unrotated, `1 0 0 1 -llx -lly cm`
  like pdfTeX. XObjects are numbered after the fonts; a page lists only the
  XObjects it paints, so documents without images serialise byte-identically
  to before (HW1 `from-v2`, HW1/HW2 `reemit` and the v2 fixtures checked).

Oracle (`tests/fixtures/images/make_oracle.py`, pdflatex; cargo never runs
TeX): 19 single-image pages (PNG RGB8/Adam7/gray 144 dpi/gray2/palette4/
palette+tRNS/RGBA8/gray+alpha/RGB16/RGBA16, JPEG RGB 96 dpi/progressive/gray/
CMYK, PDF CropBox/Rotate 90, rotated 30°/-45°/90°). `tests/images.rs` checks
our CTM at `Do` against pdfTeX's within 0.01 bp (measured max 0.00048 bp: the
producer's 1/1000 pt transform rounding), the XObject dictionary summary
(Subtype, Width, Height, BitsPerComponent, ColorSpace, palette, Filter,
Decode, SMask, BBox, Matrix), the SHA-256 of decoded samples/content, and
the page group, all equal. Ghostscript 10.07.1 at 150 dpi without
anti-aliasing (`oracle/raster.json`): 18/19 pages pixel-identical; the
CropBox form differs in one 40-pixel row (an edge on a pixel boundary under
a 1.1e-5 bp CTM difference) and is identical at 300 dpi. Tolerance: 0.01 bp
for placement, zero differing pixels at 150 dpi or, for a boundary flip, at
300 dpi.

Tests: `tests/images.rs` (above, plus rooted-read refusals for links,
`..`, missing/relative roots, stale length and hash, stale geometry, shared
XObjects and `Do` validation); `tests/exact.rs` (deterministic serialisation, verbatim decimals and
codes, CFF subset identity, bounded glyph sets, validation errors, Type 1
round trip through the reader, PFB parsing, classifier categories, and,
skipped when the tool is absent, Latin Modern rendering in CoreGraphics and
the pdflatex/xelatex oracle round trips); `tests/v2.rs` (the checked-in
`flashtex-render` envelope for fixture 01 resolved against the installed
Latin Modern, a hand-built envelope with hmtx joining, rules and colour, and
the refusals).

## Font embedding

Off by default. When enabled, the supplied OpenType font is written through
a Type0 font with `Identity-H` encoding (two bytes per glyph, written as hex
strings), a `/W` widths array with the `hmtx` advance of every glyph used, and
a `ToUnicode` CMap so text extraction and search return the original
characters. Two outline formats are handled; the parser, subsetter, and CMap
writer are hand-written (`src/truetype.rs`, `src/embed.rs`) and the crate
still has no dependencies.

**Document face.** `RenderOptions.face` / `--default-face` decides what the
embedded font is for:

- `embedded` (alias `lm`): **the embedded font is the document face.** Every
  character is looked up in it first; Symbol fills what it lacks (Greek and
  operators for Latin Modern Roman), base-14 Times fills what neither covers,
  and only a character in none of the three becomes `?` with a warning. The
  content stream for plain Latin text then uses only `/F3`. This is implied
  when `--embed-font` resolves to Latin Modern (PostScript name `LM…`).
- `times`: base-14 Times-Roman (WinAnsi) first, then Symbol, and the embedded
  font only fills the gaps. Implied for any other font, and the right choice
  for `\usepackage{times}`-style documents. With this face a Latin-only
  document embeds a font but uses no glyph from it (`/W [ ]`).

Verified on macOS with `lmroman10-regular.otf` as the face: the sample
`Latin Modern naïve — café` produces a single `/F3 12 Tf` run and no `/F1`;
PDFKit's selection bounds for `Latin Modern` at 12 pt measure 72.552 pt,
which is exactly the sum of Latin Modern's advances (6046/1000 em × 12), while
Times-Roman's advances would give 66.324 pt; the raster is visibly Computer
Modern. The test asserts the width within 0.5 pt.

- **TrueType (`.ttf`, `glyf` outlines): subset.** The glyphs used (plus
  `.notdef` and the parts of any composite glyph) are copied into a new,
  densely renumbered TrueType program and embedded as `/FontFile2` under a
  CIDFontType2 descendant with `/CIDToGIDMap /Identity`. The subset carries
  `head`, `hhea`, `maxp`, `hmtx`, `loca`, `glyf`, and, if present, `cvt `,
  `fpgm`, `prep`; table checksums and `head.checkSumAdjustment` are computed
  and `truetype::verify_checksums` reads them back in tests. A one-line
  document costs a few KB.
- **CFF OpenType (`.otf`, `OTTO`, e.g. Latin Modern): embedded whole, no
  subsetting yet.** The raw `CFF ` table is copied byte for byte into a
  `/FontFile3` `/Subtype /CIDFontType0C` stream under a CIDFontType0
  descendant; glyph ids are the font's own, and `/W` lists only the glyphs
  used. **Size cost: every document that uses the font carries the entire
  CFF table** — 61,140 bytes for `lmroman10-regular.otf`, so a one-line PDF
  is about 63 KB (measured: 63,228 bytes), and `latinmodern-math.otf` would
  add 652 KB per document. CFF subsetting (and conversion to a CID-keyed
  CFF, see below) is the planned follow-up. `/Subtype /Type1C` was tried
  first: CoreGraphics rejects it for a CIDFontType0 ("unsupported
  CIDFontType0 subtype 'Type1C'") and falls back to Helvetica; `/OpenType`
  with the whole `.otf` also renders but is ~50 KB larger per document, so
  `CIDFontType0C` is what is written. Latin Modern's CFF is not CID-keyed;
  CoreGraphics (Preview, PDFKit, `sips`) selects glyphs by CID = GID for such
  a program and both the raster and PDFKit text extraction were verified. PDF
  32000 §9.7.4.2 words the non-CID-keyed case in terms of the CFF charset, so
  other viewers may differ until the program is rewritten as CID-keyed.

Characters the embedded font also lacks still become `?` and are named in
`warnings` (one font-level warning listing them, plus the per-item warning).
Nothing is ever dropped silently. The CLI prints every warning and a final
`note: N warning(s)` line; exit status stays 0 because the PDF was written.

**Which font.** `--embed-font PATH` uses that file. `--embed-font auto` (or
setting `FLASHTEX_UNICODE_FONT` alone) uses `FLASHTEX_UNICODE_FONT` if set,
otherwise the first of these that exists (`embed::candidate_paths()` lists
them in order):

1. `$FLASHTEX_LM_DIR/lmroman10-regular.otf` if `FLASHTEX_LM_DIR` is set.
2. `<root>/<release>/texmf-dist/fonts/opentype/public/lm/lmroman10-regular.otf`
   for each root in `/usr/local/texlive`, `/opt/texlive`, `/usr/share/texlive`,
   newest release directory first (e.g. `/usr/local/texlive/2026basic/…`).
3. `/usr/share/texlive/texmf-dist/fonts/opentype/public/lm/` and
   `/usr/share/texmf/fonts/opentype/public/lm/` (Linux distribution TeX).
4. macOS only: `/System/Library/Fonts/Supplemental/Times New Roman.ttf`
   (serif, TrueType; Latin, Greek, Cyrillic; no CJK, `ℝ`, or emoji), then
   `/System/Library/Fonts/Supplemental/Arial Unicode.ttf` (sans, ~50 000
   glyphs incl. CJK; no emoji outlines).

**Provenance and licences.** Latin Modern (`lmroman10-regular.otf`, GUST
e-foundry, the OpenType form of LaTeX's default Computer Modern-derived
face) is distributed under the GUST Font License, which permits embedding and
redistribution in documents; the copy used here comes from the local TeX
Live installation, and its coverage is Latin (incl. Latin Extended) only —
no Greek, Cyrillic, or CJK, so those still warn. Times New Roman and Arial
Unicode are Apple-supplied system fonts: their licences permit use on the
Mac, but **embedding them into a PDF you redistribute is a licensing question
the user must answer for themselves.** This crate does not choose a font
unless asked, and it prints which one it embedded (`note: embedding subset
of …`). No font is committed to this repository. `.ttc` collections are
rejected with a message, not guessed at.

**Limits of the embedded route.** Glyph advances come from the font, so text
spacing inside a run is right, but the *positions* of items still come from
the compiler's own metrics, which do not know about this font. There is no
shaping: combining marks, ligature substitution, and complex scripts are
written glyph-by-glyph from the `cmap`. Emoji fonts with colour tables
(`sbix`, `COLR`) are not supported. Programs are uncompressed (no Flate).

## Limitations, stated plainly

- **Fonts: base-14 `Times-Roman` (WinAnsiEncoding) and `Symbol` only, neither
  embedded.** This is a placeholder until FlashTeX embeds its own fonts. Glyph
  shapes and advance widths come from whatever the viewer substitutes for the
  standard 14.
- **Heading weight is not reproduced.** The compiler sets headings in Times-Bold
  (`layout.rs::font_for_size`, sizes above 12pt) and measures them with bold
  widths, but runtime-v1 carries only `font_size_pt`, no font identity. This
  writer deliberately does **not** infer bold from size (issue #9: that would
  also embolden math scripts and other non-body sizes); every Times run is
  Times-Roman until the contract gains a font/weight field. Proposed to
  Commander as a runtime-v1 addition; until then headings export in regular
  weight at the right size and position, with widths slightly narrower than the
  compiler assumed.
- **Math is rendered via base-14 Symbol without embedding.** Greek (α…ω, Α…Ω,
  ς ϑ ϕ ϖ ϒ) and the operators in `src/encoding.rs::symbol_byte` (√ ∑ ∏ ∫ ∞ ± ×
  ÷ ≤ ≥ ≠ ≈ ≡ ∂ ∇ ∈ ∉ ⊂ ⊃ ⊆ ⊇ ∪ ∩ → ← ↑ ↓ ↔ ⇒ ⇐ ⇔ ∀ ∃ ¬ ∧ ∨ ′ ″ ° · … ∅ ℵ ℜ ℑ
  ℘ ⊗ ⊕ ∠ ∝ ∼ ∗ ∣ ⟨ ⟩ − ≅ ∴ ⊥ ∋ ◊) are written as single Symbol bytes. Symbol's
  glyph design does not match Times, has no bold/italic, and cannot stretch
  delimiters or radicals; the radical sign is a plain glyph with no overbar.
  Codes were transcribed from the Adobe Symbol encoding (PDF 32000-1 Annex D.5)
  and only codes the author is certain of are included; none were omitted
  from the requested list.
- **Unicode:** characters representable in WinAnsi (ASCII, Latin-1 such as
  `é ï ñ ü`, and the Windows-1252 block: `— – … € “ ” ‘ ’ Œ œ Š š Ž ž Ÿ ƒ ‰ • ™`)
  are written as their single WinAnsi byte via Times; characters Symbol covers
  are written via Symbol; with embedding enabled, anything the supplied font
  has is written through the embedded subset. Whatever remains (by default:
  Cyrillic, CJK, blackboard bold, emoji, control characters, combining marks)
  is written as `?` and reported in `warnings` with its code point. It is
  never silently dropped.
- **Text and U+2500 rules only.** Item kinds other than `text` are skipped with
  a warning. No images, general paths, links, or annotations.
- **Metrics come from the compiler, not from here.** The writer places each item
  exactly where `x_pt`/`baseline_y_pt` say. The FT-002 compiler currently
  estimates glyph widths as `0.5 × font_size`, so word gaps in the PDF will look
  uneven against real Times widths until the compiler uses real metrics. That is
  a compiler limitation; this crate does not re-measure or re-flow text.
- No compression, no object streams, no outline/bookmarks, no metadata beyond
  `/Producer` and `/Creator`. Output is plain PDF 1.4 and larger than it needs
  to be for big documents.
- Zero pages is an error (`PdfError::Invalid`), as are non-positive page sizes,
  non-positive font sizes, and non-finite coordinates.

## Export is always white

The writer takes no theme or colour input. The only colour operator emitted is
`0 g` (black text in DeviceGray) and no background rectangle is painted, so the
page renders white in every viewer regardless of what the Mac preview does in
dark mode. `tests/render.rs::export_is_white_and_theme_independent` guards this.

## Verification performed

Layout capabilities: the contract's rule example (`x 72, y 84, w 24, h 0.5`)
renders as `72 707.5 24 0.5 re f` before the following text (paint order);
a rule without `rules-v1` (negotiated or legacy) and an unknown `image` kind
under negotiation are errors naming `main.tex:0-11` / `fig.tex:3-9`; zero,
negative, and >1e6 geometry is rejected. Times hints produce `/F4`
Times-Bold, `/F5` Times-Italic, `/F6` Times-BoldItalic objects and a single
`Palatino … substituted by 'Times-Bold'` warning; Courier and Helvetica hints
select their base-14 variants (`Courier`, `Helvetica-BoldOblique`) with no
warning; a `Symbol` hint draws from
`/F2` in Symbol's built-in encoding (Times only for characters Symbol lacks)
with no warning; LM hints with LM as the
document face produce three whole-CFF font objects (`LMRoman10-Regular`,
`-Bold`, `-Italic`; 22 objects total) whose ToUnicode maps decode each run,
with `Palatino` reported as substituted by Latin Modern italic. Rasterised on
macOS, `Regular Bold Italic BoldItalic` show the four LM faces, `Times-Bold`
the base-14 bold, and a typed rule draws the bar of a stacked fraction; PDFKit
extracts the text. Legacy fixtures render byte-identically to before.

- `cargo test`: 45 tests (20 unit, 25 integration) covering the fixture's page
  count and MediaBox, a two-page synthetic result with distinct page sizes,
  multiline placement (every `Td` equals `(x_pt, height_pt - baseline_y_pt)`),
  WinAnsi encoding (`é` is byte `0xE9`, `—` is `0x97`), unrepresentable
  characters (`中`, `😀`, `ℝ`) becoming `?` with warnings, delimiter escaping,
  unsupported item kinds, bad envelopes, determinism, the CLI, and the xref
  self-check. On macOS an additional test writes the fixture PDF and asserts
  `/usr/bin/sips -g pixelWidth -g pixelHeight` reports `612` × `792`.
- Issue #9 reproduction: `tests/fixtures/math-compile-result.json` is the exact
  `flashtex-compiler` (de1020c) output for `$\frac{a}{b}+\alpha+\sqrt{x}$`
  (`tests/fixtures/math-compile-request.json`). The test asserts zero warnings,
  one `re f` rule of width 8.4pt and thickness 0.72pt at x=72 with its bottom
  edge on baseline 84.72, `α` emitted via `/F2` as byte `0x61`, `√` via `/F2` as
  `0xD6`, and every other item via `/F1` at the compiler's coordinates. A mixed
  item (`x∈ℝ→∞`) is checked to switch `/F1`/`/F2` inside one text object and
  both fonts are present in every page's `/Resources`.
- Embedding: with a font found by the same discovery the CLI uses (skipped
  with a message otherwise), `ж中ℝ😀` is rendered; the test asserts a Type0 /
  CIDFontType2 / `FontFile2` / `ToUnicode` object chain, that each character
  is either written through `/F3` with a ToUnicode entry mapping its glyph id
  back to the code point or named in a warning, that the embedded program
  re-parses with `verify_checksums` passing and glyph count equal to the
  subset's map (used glyphs + `.notdef` + composite parts), that advances
  survive subsetting, and that `sips` opens the CLI's output. Off by default
  and bad font files are errors, not silent fallbacks.
- CFF embedding: with Latin Modern found by the same search (skipped with a
  message otherwise), `ŵŷ ő 中` is rendered; the test asserts the
  Type0 / CIDFontType0 / `FontFile3 CIDFontType0C` chain with no
  `CIDToGIDMap` and no subset tag, that the embedded stream equals the
  font's `CFF ` table byte for byte (length and content), that every CID
  written is the font's own GID with a ToUnicode entry, sparse `/W` entries,
  the `中` warning, and that `auto` ranks Latin Modern before Times New
  Roman. A macOS test runs the CLI, rasterises with `CG_PDF_VERBOSE=1 sips`
  and fails if CoreGraphics reports an unsupported font program, then
  extracts the page text with PDFKit (`PDFPage.string` via PyObjC, skipped
  if PyObjC is absent) and asserts it equals the input `Latin Modern ŵŷ ő`.
- Document face: with Latin Modern as the face, `Latin Modern naïve — café`
  uses only `/F3` (no `/F1`/`/F2`), `/W` has the `hmtx` advance of every
  used glyph, ToUnicode round-trips the text; the same input with the Times
  face uses `/F1` and an empty `/W`; the issue #9 math result under the LM
  face keeps the `re f` rule, sets `a b + x` in LM, `α` in Symbol, and `√`
  in LM (which has a radical glyph) with zero warnings; a macOS test runs
  the CLI without `--default-face`, requires the "as the document face"
  note, rasterises cleanly, and checks PDFKit's selection width for
  `Latin Modern` against the LM advances (±0.5 pt) and away from Times.
- Manual: the fixture and a two-page Unicode sample were rendered, opened by
  `sips` (`format: pdf`, `pixelWidth: 612.000`, `pixelHeight: 792.000`), and
  rasterised to PNG; the heading, three baselines, accented characters, escaped
  parentheses, the `?` substitution, a bottom-margin line, and page two all
  appeared where expected on a white page. The issue #9 math PDF rasterised by
  macOS shows `a` over a drawn bar over `b`, then `+α+√x`, with no `?`.
- Embedding, rasterised by macOS: with Arial Unicode, `Latin café — Greek αβγ
  — Cyrillic жизнь — CJK 中文 — ℝ ∫ 😀` shows every script and `ℝ`, with `?`
  only for the emoji; `Ǆǅ Ŵŷ ő` (composite glyphs) render correctly. With
  Times New Roman the same line shows `?` for CJK and `ℝ` exactly as warned.

## Integration with the Mac app

The Rust worker (`crates/compiler`) still reports `pdf_path: null`, as the
contract permits. To attach export:

1. In the worker, after producing a `compile_result`, call
   `flashtex_pdf::render_envelope` (or `render_pdf` on the already-built pages),
   write the bytes to a per-project temporary file, and set `pdf_path` to that
   path. Forward `warnings` as `warning` diagnostics with `source: null` so the
   UI shows them; do not drop them.
2. Alternatively, until the worker is wired, `apps/mac` can shell out to the
   `flashtex-pdf` binary next to `flashtex-compiler`, feeding it the
   `compile_result` line it already holds and reading `--out`.
3. The Mac "Export PDF" action then copies `pdf_path` to the user's chosen
   location. The export never depends on the preview theme.

Neither step changes the runtime-v1 contract; `pdf_path` already exists for
this purpose. Adding a `pdf_warnings` field would be a contract change and
belongs to Commander.
