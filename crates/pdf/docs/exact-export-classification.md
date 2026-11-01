# Exact export against the declared reference profile: classification of remaining differences

Agent `mac-pdf` (parent `mac-claude-a`, machine `mac-m1max-a`), branch
`agent/mac-pdf/exact-export`. Written 2026-09-12 from an actual run on this
Mac; every number below was produced by the commands in "Reproduction".

## What was measured

The exact export route (`flashtex_pdf::exact`, GitHub issue #25) takes glyph
runs by original glyph id, typed operators, and content streams whose
decimal operands are written verbatim, embeds font programs byte for byte,
and writes through the crate's one container implementation. To test it
against the reference profile declared by the visual-corpus harness
(`tests/visual-corpus/harness/reference-profile.json` and
`render_reference.sh` on `origin/agent/mac-visual-oracle/reference-raster`),
every corpus fixture was rendered by the oracle engines and the resulting
PDF was **re-emitted through the exact API** (`flashtex-pdf-exact reemit`):
page sizes, decoded content streams (as `Content::Verbatim`), and every font
resource with its program bytes, widths, encoding, descriptor, ToUnicode and
CIDSet were carried into an `ExactDocument` and rendered by `render_exact`.
The reference and the re-emitted file were then compared with
`flashtex-pdf-exact classify` and rasterised by CoreGraphics at 144 dpi.

This measures the *container and API*: whether the exact route can carry a
real producer's glyph selection, decimals, rules and font programs without
loss. It does **not** measure whether FlashTeX's compiler produces those
streams; that is upstream of this crate (rendering-core / compiler owners)
and is not claimed here.

## Reference profile

| item | value |
|---|---|
| TeX Live | MacTeX 2026 full, `/usr/local/texlive/2026/bin/universal-darwin` (oracle only, never in the product path) |
| pdflatex | pdfTeX 3.141592653-2.6-1.40.29 (TeX Live 2026); output PDF 1.7, xref stream, one object stream, `FlateDecode` on every stream, Type 1 subsets (`FontFile` with `Length1/2/3`), `TJ` kerning arrays, rules as stroked lines (`q … cm [] 0 d 0 J w m l S Q`) |
| xelatex | XeTeX with xdvipdfmx 20260317; output PDF 1.7, xref stream, object stream, `FlateDecode`, CID-keyed CFF subsets as `Type0`/`Identity-H`/`CIDFontType0C` (Latin Modern OTF), `CIDFontType2` (Times New Roman TrueType), `Type1C` for Computer Modern math, `CIDSet` and ToUnicode streams |
| preambles | the harness's four: pdflatex `times` (URW Nimbus Roman via psnfss) and `lmodern`; xelatex fontspec `Times New Roman` and Latin Modern by explicit OTF path |
| fixtures | all 18 in `tests/visual-corpus/harness/fixtures/` (bodies only; the harness replaces the preamble) |
| rasteriser | CoreGraphics (`CGContextDrawPDFPage`, PyObjC Quartz) at 144 dpi, RGBA, white ground; pixel bytes compared exactly |
| writer | `flashtex-pdf-exact` built from this branch at `b075468` (`cargo build --release`) |

## Result

72 references (18 fixtures × 4 engine/preamble variants), 92 pages, 234
embedded font programs:

| measure | result |
|---|---|
| content streams (decoded) | **byte-identical on all 92 pages**, including `TJ` arrays with kerning integers, `\002`-style one-byte codes, two-byte CIDs, and the stroked-line rules with `0.398 w` |
| font programs | **byte-identical (SHA-256) for all 234**: Type 1 (`Length1/2/3` preserved), CID-keyed CFF (`CIDFontType0C`), TrueType (`FontFile2`), `Type1C` |
| font metadata | no differences: `Widths`/`W` and `DW` (compared semantically), `Differences` encodings, descriptors including `AvgWidth`, `Style/Panose`, `CharSet`, `XHeight`, `CIDSet` (by content), ToUnicode (carried verbatim) |
| page geometry | identical MediaBox on every page; per-page `/Resources` lists |
| CoreGraphics raster 144 dpi | **pixel-identical on all 92 pages** |
| classifier exit status | 0 on all 72 |
| remaining categories | exactly `ObjectLayout, Compression, DocumentIdentity` on all 72 |

The full per-fixture table is at the end of this file.

## The remaining differences, classified

Every difference the classifier still reports falls into one of three
categories. None of them changes what a viewer draws (the raster identity
above is the evidence), and each is a deliberate property of this writer
rather than an approximation:

1. **ObjectLayout.** The references are PDF 1.7 with a cross-reference
   stream and one object stream; this writer emits PDF 1.4 with a classic
   xref table and no object streams, and numbers objects in its own fixed
   order (catalog, pages, info, page/content pairs, then fonts in
   resource-name order). Object numbers are not content; the reader used
   by the classifier resolves references, so a 1:1 mapping is verified
   rather than assumed.
2. **Compression.** The references apply `FlateDecode` to every stream;
   this writer applies no filter. The comparison is on decoded bytes. An
   in-tree inflate decoder was added to *read* the references; the writer
   still does not compress (a future writer-side Flate encoder would be a
   pure size optimisation).
3. **DocumentIdentity.** The references carry a trailer `/ID`, `/CreationDate`,
   `/ModDate`, `/Producer (pdfTeX-1.40.29)` or `(xdvipdfmx (20260317))`,
   `/Creator`, `/PTEX.Fullbanner`, `/Trapped`. This writer emits no `/ID` and
   no dates so that the same input always gives the same bytes (tested), and
   names itself as producer.

Categories that could have appeared but did not, on any fixture:
`PageGeometry`, `ContentFormatting`, `ContentOperands`, `ContentOperators`,
`ContentUnsupported`, `FontProgram`, `FontMetadata`, `FontResources`.

Raw-byte versus raster, in one line: the files are **not** raw-byte
identical (layout, compression, identity) and **are** identical at the
level the profile permits comparing (decoded content bytes, font program
bytes, dictionaries) and at the raster.

## What this does and does not establish

- Established: the exact route loses nothing from a real producer's page
  operators, decimal operands, one- and two-byte glyph codes, Type 1 / CFF /
  TrueType programs and font dictionaries; the reader, re-emit and classifier
  are independent enough of the writer to catch a wrong offset, a reformatted
  number, a substituted glyph or a re-subset font (each is a test case).
- Established: the crate's own CFF subsetter (`crate::cff`) produces CID-keyed
  programs in the same form xdvipdfmx uses, keeps original GIDs as CIDs and
  charstring bytes unchanged, and CoreGraphics renders them (test with Latin
  Modern; skipped where it is not installed).
- Not established: that FlashTeX's compiler/rendering-core emits streams
  equal to pdfTeX's for these fixtures. The runtime-v1 route remains as
  documented in the README (base-14 placeholder fonts, three-decimal
  formatting); the corpus report for that route is `corpus-report.md`.
- Not established: parity in viewers other than CoreGraphics. The CID-keyed
  CFF form is the one dvipdfmx has written for years, so other viewers are
  expected to agree, but no other viewer was run.

## Contrast: the runtime-v1 route against the same reference

For scale, the same classifier was run on fixture 01 produced the existing
way (compiler `origin/main` → runtime-v1 `compile_result` → `flashtex-pdf
--embed-font auto`, Latin Modern as the document face) against the pdflatex
`times` reference. Result (informative, not a test): `ContentOperators`
(5 operators vs 66: one `Tj` per word at compiler coordinates, `72 708 Td`
vs `72 708.045 Td`, `/F3 12 Tf` vs `/F44 11.9552 Tf`, no `TJ` kerning),
`FontResources` (`/F1 /F2 /F3` vs `/F44`), plus the same
`ObjectLayout`/`Compression`/`DocumentIdentity`; CoreGraphics raster: 6,257
of 1,938,816 pixels differ; classifier exit 3. That is the expected shape of
the gap: it lives in what the producer hands the writer (glyph selection,
metrics, positions), which is exactly what the exact route lets a producer
state precisely, and the container itself adds nothing to it.

## Reproduction

Oracle-only steps (MacTeX and PyObjC needed; nothing here is in the product
path). The scratch scripts live outside the repository; the equivalent test
in the crate is `tests/exact.rs::pdflatex_reference_reemits_with_identical_content_and_font_programs`
and `::xelatex_reference_with_cid_keyed_cff_reemits_identically`, which run
the same round trip on a small document and assert the same categories.

```sh
cd crates/pdf && cargo build --release
# render a fixture with the harness preamble (see render_reference.sh), then
target/release/flashtex-pdf-exact dump    ref/main.pdf            # pages, fonts, operators
target/release/flashtex-pdf-exact reemit  ref/main.pdf out.pdf    # through the exact API
target/release/flashtex-pdf-exact classify ref/main.pdf out.pdf   # exit 0 iff content ops + programs identical
```

`cargo test` (71 tests: 35 unit, 11 `tests/exact.rs`, 25 `tests/render.rs`)
and `cargo clippy --all-targets` are clean at `b075468`.

## Per-fixture table

Columns: classify exit status; number of pages whose decoded content stream
is byte-identical; embedded font programs (SHA-256 identical, differing);
CoreGraphics raster comparison per page; categories the classifier still
reports.

| fixture | engine/preamble | classify | pages byte-identical content | font programs (same, differ) | CoreGraphics raster 144 dpi | remaining categories |
|---|---|---|---|---|---|---|
| 01-plain-paragraph | times | exit 0 | 1 | 1 same, 0 differ | p1 identical | ObjectLayout, Compression, DocumentIdentity |
| 01-plain-paragraph | lm | exit 0 | 1 | 1 same, 0 differ | p1 identical | ObjectLayout, Compression, DocumentIdentity |
| 01-plain-paragraph | xelatex-times | exit 0 | 1 | 1 same, 0 differ | p1 identical | ObjectLayout, Compression, DocumentIdentity |
| 01-plain-paragraph | xelatex-lm | exit 0 | 1 | 1 same, 0 differ | p1 identical | ObjectLayout, Compression, DocumentIdentity |
| 02-wrapping-paragraph | times | exit 0 | 1 | 1 same, 0 differ | p1 identical | ObjectLayout, Compression, DocumentIdentity |
| 02-wrapping-paragraph | lm | exit 0 | 1 | 1 same, 0 differ | p1 identical | ObjectLayout, Compression, DocumentIdentity |
| 02-wrapping-paragraph | xelatex-times | exit 0 | 1 | 1 same, 0 differ | p1 identical | ObjectLayout, Compression, DocumentIdentity |
| 02-wrapping-paragraph | xelatex-lm | exit 0 | 1 | 1 same, 0 differ | p1 identical | ObjectLayout, Compression, DocumentIdentity |
| 03-section-heading | times | exit 0 | 1 | 2 same, 0 differ | p1 identical | ObjectLayout, Compression, DocumentIdentity |
| 03-section-heading | lm | exit 0 | 1 | 2 same, 0 differ | p1 identical | ObjectLayout, Compression, DocumentIdentity |
| 03-section-heading | xelatex-times | exit 0 | 1 | 2 same, 0 differ | p1 identical | ObjectLayout, Compression, DocumentIdentity |
| 03-section-heading | xelatex-lm | exit 0 | 1 | 2 same, 0 differ | p1 identical | ObjectLayout, Compression, DocumentIdentity |
| 04-bold-emph | times | exit 0 | 1 | 4 same, 0 differ | p1 identical | ObjectLayout, Compression, DocumentIdentity |
| 04-bold-emph | lm | exit 0 | 1 | 4 same, 0 differ | p1 identical | ObjectLayout, Compression, DocumentIdentity |
| 04-bold-emph | xelatex-times | exit 0 | 1 | 4 same, 0 differ | p1 identical | ObjectLayout, Compression, DocumentIdentity |
| 04-bold-emph | xelatex-lm | exit 0 | 1 | 4 same, 0 differ | p1 identical | ObjectLayout, Compression, DocumentIdentity |
| 05-unicode | times | exit 0 | 1 | 1 same, 0 differ | p1 identical | ObjectLayout, Compression, DocumentIdentity |
| 05-unicode | lm | exit 0 | 1 | 1 same, 0 differ | p1 identical | ObjectLayout, Compression, DocumentIdentity |
| 05-unicode | xelatex-times | exit 0 | 1 | 1 same, 0 differ | p1 identical | ObjectLayout, Compression, DocumentIdentity |
| 05-unicode | xelatex-lm | exit 0 | 1 | 1 same, 0 differ | p1 identical | ObjectLayout, Compression, DocumentIdentity |
| 06-math-inline | times | exit 0 | 1 | 5 same, 0 differ | p1 identical | ObjectLayout, Compression, DocumentIdentity |
| 06-math-inline | lm | exit 0 | 1 | 5 same, 0 differ | p1 identical | ObjectLayout, Compression, DocumentIdentity |
| 06-math-inline | xelatex-times | exit 0 | 1 | 5 same, 0 differ | p1 identical | ObjectLayout, Compression, DocumentIdentity |
| 06-math-inline | xelatex-lm | exit 0 | 1 | 5 same, 0 differ | p1 identical | ObjectLayout, Compression, DocumentIdentity |
| 07-math-display | times | exit 0 | 1 | 6 same, 0 differ | p1 identical | ObjectLayout, Compression, DocumentIdentity |
| 07-math-display | lm | exit 0 | 1 | 6 same, 0 differ | p1 identical | ObjectLayout, Compression, DocumentIdentity |
| 07-math-display | xelatex-times | exit 0 | 1 | 6 same, 0 differ | p1 identical | ObjectLayout, Compression, DocumentIdentity |
| 07-math-display | xelatex-lm | exit 0 | 1 | 6 same, 0 differ | p1 identical | ObjectLayout, Compression, DocumentIdentity |
| 08-two-page | times | exit 0 | 3 | 3 same, 0 differ | p1 identical; p2 identical; p3 identical | ObjectLayout, Compression, DocumentIdentity |
| 08-two-page | lm | exit 0 | 3 | 3 same, 0 differ | p1 identical; p2 identical; p3 identical | ObjectLayout, Compression, DocumentIdentity |
| 08-two-page | xelatex-times | exit 0 | 3 | 3 same, 0 differ | p1 identical; p2 identical; p3 identical | ObjectLayout, Compression, DocumentIdentity |
| 08-two-page | xelatex-lm | exit 0 | 3 | 3 same, 0 differ | p1 identical; p2 identical; p3 identical | ObjectLayout, Compression, DocumentIdentity |
| 09-mixed-document | times | exit 0 | 1 | 6 same, 0 differ | p1 identical | ObjectLayout, Compression, DocumentIdentity |
| 09-mixed-document | lm | exit 0 | 1 | 6 same, 0 differ | p1 identical | ObjectLayout, Compression, DocumentIdentity |
| 09-mixed-document | xelatex-times | exit 0 | 1 | 6 same, 0 differ | p1 identical | ObjectLayout, Compression, DocumentIdentity |
| 09-mixed-document | xelatex-lm | exit 0 | 1 | 6 same, 0 differ | p1 identical | ObjectLayout, Compression, DocumentIdentity |
| 10-unicode-paragraph | times | exit 0 | 1 | 1 same, 0 differ | p1 identical | ObjectLayout, Compression, DocumentIdentity |
| 10-unicode-paragraph | lm | exit 0 | 1 | 2 same, 0 differ | p1 identical | ObjectLayout, Compression, DocumentIdentity |
| 10-unicode-paragraph | xelatex-times | exit 0 | 1 | 1 same, 0 differ | p1 identical | ObjectLayout, Compression, DocumentIdentity |
| 10-unicode-paragraph | xelatex-lm | exit 0 | 1 | 1 same, 0 differ | p1 identical | ObjectLayout, Compression, DocumentIdentity |
| 11-nested-lists | times | exit 0 | 1 | 1 same, 0 differ | p1 identical | ObjectLayout, Compression, DocumentIdentity |
| 11-nested-lists | lm | exit 0 | 1 | 2 same, 0 differ | p1 identical | ObjectLayout, Compression, DocumentIdentity |
| 11-nested-lists | xelatex-times | exit 0 | 1 | 1 same, 0 differ | p1 identical | ObjectLayout, Compression, DocumentIdentity |
| 11-nested-lists | xelatex-lm | exit 0 | 1 | 1 same, 0 differ | p1 identical | ObjectLayout, Compression, DocumentIdentity |
| 12-justified-paragraphs | times | exit 0 | 1 | 1 same, 0 differ | p1 identical | ObjectLayout, Compression, DocumentIdentity |
| 12-justified-paragraphs | lm | exit 0 | 1 | 1 same, 0 differ | p1 identical | ObjectLayout, Compression, DocumentIdentity |
| 12-justified-paragraphs | xelatex-times | exit 0 | 1 | 1 same, 0 differ | p1 identical | ObjectLayout, Compression, DocumentIdentity |
| 12-justified-paragraphs | xelatex-lm | exit 0 | 1 | 1 same, 0 differ | p1 identical | ObjectLayout, Compression, DocumentIdentity |
| 13-math-display-rich | times | exit 0 | 1 | 8 same, 0 differ | p1 identical | ObjectLayout, Compression, DocumentIdentity |
| 13-math-display-rich | lm | exit 0 | 1 | 8 same, 0 differ | p1 identical | ObjectLayout, Compression, DocumentIdentity |
| 13-math-display-rich | xelatex-times | exit 0 | 1 | 8 same, 0 differ | p1 identical | ObjectLayout, Compression, DocumentIdentity |
| 13-math-display-rich | xelatex-lm | exit 0 | 1 | 8 same, 0 differ | p1 identical | ObjectLayout, Compression, DocumentIdentity |
| 14-math-inline-dense | times | exit 0 | 1 | 7 same, 0 differ | p1 identical | ObjectLayout, Compression, DocumentIdentity |
| 14-math-inline-dense | lm | exit 0 | 1 | 7 same, 0 differ | p1 identical | ObjectLayout, Compression, DocumentIdentity |
| 14-math-inline-dense | xelatex-times | exit 0 | 1 | 7 same, 0 differ | p1 identical | ObjectLayout, Compression, DocumentIdentity |
| 14-math-inline-dense | xelatex-lm | exit 0 | 1 | 7 same, 0 differ | p1 identical | ObjectLayout, Compression, DocumentIdentity |
| 15-three-page-sections | times | exit 0 | 3 | 6 same, 0 differ | p1 identical; p2 identical; p3 identical | ObjectLayout, Compression, DocumentIdentity |
| 15-three-page-sections | lm | exit 0 | 3 | 6 same, 0 differ | p1 identical; p2 identical; p3 identical | ObjectLayout, Compression, DocumentIdentity |
| 15-three-page-sections | xelatex-times | exit 0 | 3 | 6 same, 0 differ | p1 identical; p2 identical; p3 identical | ObjectLayout, Compression, DocumentIdentity |
| 15-three-page-sections | xelatex-lm | exit 0 | 3 | 6 same, 0 differ | p1 identical; p2 identical; p3 identical | ObjectLayout, Compression, DocumentIdentity |
| 16-heading-page-break | times | exit 0 | 2 | 3 same, 0 differ | p1 identical; p2 identical | ObjectLayout, Compression, DocumentIdentity |
| 16-heading-page-break | lm | exit 0 | 2 | 3 same, 0 differ | p1 identical; p2 identical | ObjectLayout, Compression, DocumentIdentity |
| 16-heading-page-break | xelatex-times | exit 0 | 2 | 3 same, 0 differ | p1 identical; p2 identical | ObjectLayout, Compression, DocumentIdentity |
| 16-heading-page-break | xelatex-lm | exit 0 | 2 | 3 same, 0 differ | p1 identical; p2 identical | ObjectLayout, Compression, DocumentIdentity |
| 17-apostrophes | times | exit 0 | 1 | 1 same, 0 differ | p1 identical | ObjectLayout, Compression, DocumentIdentity |
| 17-apostrophes | lm | exit 0 | 1 | 1 same, 0 differ | p1 identical | ObjectLayout, Compression, DocumentIdentity |
| 17-apostrophes | xelatex-times | exit 0 | 1 | 1 same, 0 differ | p1 identical | ObjectLayout, Compression, DocumentIdentity |
| 17-apostrophes | xelatex-lm | exit 0 | 1 | 1 same, 0 differ | p1 identical | ObjectLayout, Compression, DocumentIdentity |
| 18-ligatures | times | exit 0 | 1 | 1 same, 0 differ | p1 identical | ObjectLayout, Compression, DocumentIdentity |
| 18-ligatures | lm | exit 0 | 1 | 1 same, 0 differ | p1 identical | ObjectLayout, Compression, DocumentIdentity |
| 18-ligatures | xelatex-times | exit 0 | 1 | 1 same, 0 differ | p1 identical | ObjectLayout, Compression, DocumentIdentity |
| 18-ligatures | xelatex-lm | exit 0 | 1 | 1 same, 0 differ | p1 identical | ObjectLayout, Compression, DocumentIdentity |
