# rendering-v2 → exact export: measured gap against the pdflatex-lmodern reference

Agent `mac-pdf` (parent `mac-claude-a`, `mac-m1max-a`), branch
`agent/mac-pdf/v2-adapter`. Written 2026-09-12 from an actual run; every
number below came out of the commands in "Reproduction". This is the first
end-to-end path that feeds the exact route from FlashTeX's own producer, and
the deliverable is the **measured gap per category**, not parity: the
upstream layout is not pdfTeX's and nothing here claims it is.

## Pipeline under test

```
fixture body  ──►  flashtex-render --v2 --secnumdepth 0  ──►  display_list envelope (rendering-v2, bp_2pow20 ticks)
              ──►  flashtex-pdf-exact from-v2 LIST.json --out OUT.pdf
                     glyph_run → GlyphRun/PlacedGlyph by original GID, every glyph at its exact tick origin
                     fonts by content hash → lmroman*.otf / latinmodern-math.otf → GID-preserving CID-keyed CFF subset
                     rule → Op::rule; page size from ticks
              ──►  flashtex-pdf-exact classify REF.pdf OUT.pdf  +  CoreGraphics 144 dpi raster comparison
```

| item | value |
|---|---|
| producer | `flashtex-render` built from `origin/agent/mac-render-pipeline/unified` at `d556519` (`git archive` into a scratch directory, `cargo build --release --bin flashtex-render`, as `tools/typing-bench/run.sh` does), invoked with `--secnumdepth 0 --v2`, one request per fixture, body from `\begin{document}` only (the harness strips the preamble). The first run of this report used `ba5611f`; the tip changed only the math fixtures (see "Producer tip d556519") |
| adapter | `flashtex-pdf-exact from-v2` from this branch (`crates/pdf/src/v2.rs`), default font search (MacTeX 2026 Latin Modern directories) |
| reference | MacTeX 2026 pdflatex (pdfTeX 1.40.29) with the harness `lm` preamble (`\documentclass[12pt]{article}`, T1 fontenc, `lmodern`, `geometry` margin 1in, `\parindent` 0, `secnumdepth` 0, `\pagestyle{empty}`); the same references as `exact-export-classification.md` |
| comparison | `flashtex-pdf-exact classify` (categories as in `exact-export-classification.md`; fonts unmatched by resource name are paired by base font name so program/metadata gaps are measured) and CoreGraphics `CGContextDrawPDFPage` at 144 dpi. `% px differ` counts any RGB difference (anti-aliasing included); `ref-only`/`ours-only` count pixels that carry ink on one side only, i.e. differences a reader can see |

## Result, per fixture

| fixture | from-v2 | content | fonts | page geometry | CoreGraphics 144 dpi vs pdflatex-lmodern | pipeline diagnostics |
|---|---|---|---|---|---|---|
| 01-plain-paragraph | 1 page(s), 13 glyph run(s), 55 glyph(s) (0 continued at the natural advance, 0 with an exact TJ kern), 0 rule(s), 9595 bytes | ContentOperators | FontMetadata; FontProgram | same | pages 1/1; p1 0.01% px differ; ink 4355 vs 4355, ref-only 0, ours-only 0 | none |
| 02-wrapping-paragraph | 1 page(s), 210 glyph run(s), 896 glyph(s) (0 continued at the natural advance, 0 with an exact TJ kern), 0 rule(s), 76903 bytes | ContentOperators | FontMetadata; FontProgram | same | pages 1/1; p1 0.16% px differ; ink 72584 vs 72578, ref-only 74, ours-only 68 | none |
| 03-section-heading | 1 page(s), 15 glyph run(s), 79 glyph(s) (0 continued at the natural advance, 0 with an exact TJ kern), 0 rule(s), 14765 bytes | ContentOperators | FontMetadata; FontProgram | same | pages 1/1; p1 0.02% px differ; ink 9373 vs 9372, ref-only 12, ours-only 11 | none |
| 04-bold-emph | 1 page(s), 12 glyph run(s), 54 glyph(s) (3 continued at the natural advance, 0 with an exact TJ kern), 0 rule(s), 17232 bytes | ContentOperators | FontMetadata; FontProgram | same | pages 1/1; p1 0.01% px differ; ink 4863 vs 4863, ref-only 0, ours-only 0 | none |
| 05-unicode | 1 page(s), 11 glyph run(s), 46 glyph(s) (0 continued at the natural advance, 0 with an exact TJ kern), 0 rule(s), 8859 bytes | ContentOperators | FontMetadata; FontProgram | same | pages 1/1; p1 0.01% px differ; ink 3904 vs 3904, ref-only 0, ours-only 0 | none |
| 06-math-inline | 1 page(s), 13 glyph run(s), 45 glyph(s) (0 continued at the natural advance, 0 with an exact TJ kern), 2 rule(s), 11694 bytes | ContentOperators | FontMetadata; FontProgram; FontResources | same | pages 1/1; p1 0.03% px differ; ink 3432 vs 3445, ref-only 38, ours-only 51 | none |
| 07-math-display | 1 page(s), 10 glyph run(s), 47 glyph(s) (0 continued at the natural advance, 0 with an exact TJ kern), 1 rule(s), 11536 bytes | ContentOperators | FontMetadata; FontProgram; FontResources | same | pages 1/1; p1 0.07% px differ; ink 3889 vs 3770, ref-only 306, ours-only 187 | none |
| 08-two-page | 3 page(s), 1800 glyph run(s), 7680 glyph(s) (0 continued at the natural advance, 0 with an exact TJ kern), 0 rule(s), 608568 bytes | ContentOperators | FontMetadata; FontProgram | same | pages 3/3; p1 0.56% px differ; ink 254503 vs 254497, ref-only 240, ours-only 234; p2 0.55% px differ; ink 254661 vs 254649, ref-only 232, ours-only 220; p3 0.24% px differ; ink 112626 vs 112614, ref-only 108, ours-only 96 | none |
| 09-mixed-document | 1 page(s), 50 glyph run(s), 206 glyph(s) (2 continued at the natural advance, 0 with an exact TJ kern), 3 rule(s), 32849 bytes | ContentOperators | FontMetadata; FontProgram; FontResources | same | pages 1/1; p1 0.13% px differ; ink 17757 vs 17809, ref-only 147, ours-only 199 | none |
| 10-unicode-paragraph | 1 page(s), 81 glyph run(s), 425 glyph(s) (0 continued at the natural advance, 0 with an exact TJ kern), 0 rule(s), 47876 bytes | ContentOperators | FontMetadata; FontProgram; FontResources | same | pages 1/1; p1 0.16% px differ; ink 34516 vs 34285, ref-only 822, ours-only 591 | missing_glyph |
| 11-nested-lists | 1 page(s), 29 glyph run(s), 147 glyph(s) (0 continued at the natural advance, 0 with an exact TJ kern), 0 rule(s), 17444 bytes | ContentOperators | FontMetadata; FontProgram; FontResources | same | pages 1/1; p1 1.01% px differ; ink 11160 vs 11146, ref-only 9457, ours-only 9443 | none |
| 12-justified-paragraphs | 1 page(s), 360 glyph run(s), 1536 glyph(s) (0 continued at the natural advance, 0 with an exact TJ kern), 0 rule(s), 126682 bytes | ContentOperators | FontMetadata; FontProgram | same | pages 1/1; p1 0.28% px differ; ink 124408 vs 124396, ref-only 148, ours-only 136 | none |
| 13-math-display-rich | 1 page(s), 17 glyph run(s), 72 glyph(s) (0 continued at the natural advance, 0 with an exact TJ kern), 3 rule(s), 17022 bytes | ContentOperators | FontMetadata; FontProgram; FontResources | same | pages 1/1; p1 0.29% px differ; ink 5244 vs 5700, ref-only 2289, ours-only 2745 | compiler, compiler |
| 14-math-inline-dense | 1 page(s), 71 glyph run(s), 147 glyph(s) (0 continued at the natural advance, 0 with an exact TJ kern), 4 rule(s), 27588 bytes | ContentOperators | FontMetadata; FontProgram; FontResources | same | pages 1/1; p1 0.48% px differ; ink 10622 vs 10890, ref-only 2974, ours-only 3242 | compiler, overfull_hbox |
| 15-three-page-sections | 3 page(s), 1806 glyph run(s), 7695 glyph(s) (0 continued at the natural advance, 0 with an exact TJ kern), 0 rule(s), 611989 bytes | ContentOperators | FontMetadata; FontProgram | same | pages 3/3; p1 0.46% px differ; ink 208434 vs 208422, ref-only 240, ours-only 228; p2 0.46% px differ; ink 208508 vs 208496, ref-only 240, ours-only 228; p3 0.46% px differ; ink 208516 vs 208504, ref-only 240, ours-only 228 | compiler, compiler |
| 16-heading-page-break | 2 page(s), 962 glyph run(s), 4109 glyph(s) (0 continued at the natural advance, 0 with an exact TJ kern), 0 rule(s), 332089 bytes | ContentOperators | FontMetadata; FontProgram | same | pages 2/2; p1 0.56% px differ; ink 254503 vs 254497, ref-only 240, ours-only 234; p2 0.17% px differ; ink 80091 vs 80080, ref-only 73, ours-only 62 | none |
| 17-apostrophes | 1 page(s), 25 glyph run(s), 132 glyph(s) (0 continued at the natural advance, 0 with an exact TJ kern), 0 rule(s), 17238 bytes | ContentOperators | FontMetadata; FontProgram | same | pages 1/1; p1 0.01% px differ; ink 9482 vs 9482, ref-only 0, ours-only 0 | none |
| 18-ligatures | 1 page(s), 36 glyph run(s), 170 glyph(s) (0 continued at the natural advance, 0 with an exact TJ kern), 0 rule(s), 19958 bytes | ContentOperators | FontMetadata; FontProgram | same | pages 1/1; p1 0.04% px differ; ink 14798 vs 14796, ref-only 34, ours-only 32 | none |

All 18 fixtures went through `from-v2` without error (31 font subsets, all
`CffSubset`, none embedded whole), every output passed the xref self-check,
page count and MediaBox match the reference on every page, and PDFKit
extracts the same text as from the reference (checked on 01 and 18; the fi/ffi
ligatures come back as letters through the cluster ToUnicode).

## Reading the categories

**ContentOperators on every fixture** is expected and is two things at once:

1. *Representation.* pdfTeX writes one `TJ` array per line with kerning
   integers and rounds every operand (`11.9552 Tf`, `72 708.045 Td`); the
   exact route writes the pipeline's authoritative absolute origin for every
   glyph (`1 0 0 1 72 708.0448322296142578125 Tm`, `/F1 11.9551677703857421875
   Tf`, 5 operators vs 149 on 01). Both are exact for their producer; the
   decimals are the pipeline's ticks written verbatim, which is what the
   route promises. No glyph joined a string on real output (the pipeline's
   `advance_x` is TFM-based and never equals `hmtx × size` exactly), so the
   streams are larger than pdfTeX's; that is a size cost, not a fidelity one.
2. *Layout.* Where the pipeline lays out differently from LaTeX, the
   operator streams differ in substance; the raster column says where.

**FontProgram/FontMetadata on every fixture**: pdflatex embeds Latin Modern
as Type 1 (`lmr12` subset, `FontFile` Length1/2/3, `/Differences`
encoding); the exact route embeds the OpenType CFF face as a CID-keyed subset
(`FontFile3 /CIDFontType0C`, `Identity-H`, CID = original GID). Same design,
different program format and dictionaries. Program bytes therefore never
match, by construction.

**FontResources only on the math fixtures (06, 07, 09, 13, 14) and on 10/11**:
pdflatex sets math in Computer Modern Type 1 (`CMMI`, `CMSY`, `CMR`) while the
pipeline uses `latinmodern-math.otf`, genuinely different font sets. On 10 and
11 the reference carries a *second* font dictionary for the same
`LMRoman12-Regular` face (pdfTeX uses one dictionary per 8-bit encoding, and
codes 128–255 such as the list bullet on 11 and the Latin-Extended letters on
10 live in another one), whereas the exact route has exactly one resource per
face; the classifier reports the unpaired second dictionary.

**Raster, the honest measure of the layout gap** (`ref-only` / `ours-only` ink
pixels at 144 dpi, page 1 unless noted):

| fixtures | ink-only pixels | reading |
|---|---|---|
| 01, 04, 05, 17 | 0 / 0 | binary-ink identical; the `% px differ` is anti-aliasing from sub-thousandth-point position/size differences |
| 02, 03, 08 (3 pp), 12, 15 (3 pp), 16 (2 pp), 18 | 11–240 per page out of 9k–255k ink pixels (≤ 0.1%) | stray edge pixels, no visible layout difference; consistent with the pipeline's own word-box evidence (dx ≤ 0.09 pt) |
| 06 | 162 / 170 | inline math: Latin Modern Math glyphs vs Computer Modern glyphs, same positions to the pixel elsewhere |
| 07, 09, 13, 14 | 1.4k–6.2k | display math: different math fonts plus pipeline layout differences (13: `\left`/`\right` unsupported, 14: `\nu` unsupported and an overfull line, both reported by the pipeline) |
| 10 | 822 / 591 | one `missing_glyph` (U+01C5) reported by the pipeline; text otherwise aligned |
| 11 | 9457 / 9443, best vertical registration 8 px | lists are not implemented upstream (`\item` set as plain paragraphs), so the whole body shifts |

For comparison, the same fixture 01 through the runtime-v1 route
(`flashtex-compiler` → `flashtex-pdf --embed-font auto`) differs from the
reference in 6,257 pixels (`exact-export-classification.md`); through
`flashtex-render --v2` → `from-v2` it differs in 0 ink pixels and 0.01% of
anti-aliased pixels. The pipeline's own `docs/oracle-evidence.md` measured
13.5% differing pixels on the dense pages through the v1 export route and
attributed it to ink weight; through the exact route the dense pages show
240 ink-only pixels out of 254,503 (0.09%) with ink counts within 6 pixels of
the reference, so that attribution no longer holds for this route.

## Follow-up: subroutine pruning and exact `TJ` kerning (before/after)

Two changes were made after the first run and the 18 fixtures were re-run
with the same producer build, references and comparison:

1. **CFF subsetting now prunes the subroutine and string INDEXes.** Only
   the local/global subroutines the retained glyphs reach (transitively)
   are kept, renumbered, with every `callsubr`/`callgsubr` operand
   rewritten; the String INDEX keeps only the Top DICT's own strings (a
   CID-keyed charset carries no glyph names). CID = original GID is
   unchanged and the identity test now compares the *inlined* charstring
   of every retained glyph before and after (byte-identical), plus the
   unpruned form (raw-identical charstrings). `latinmodern-math.otf`:
   7-glyph subset 109,797 → 1,753 bytes; `lmroman12-regular.otf`: 18 glyphs
   23,130 → 2,634 bytes. CoreGraphics raster of the 06 export before/after
   pruning: pixel-identical; PDFKit text identical.
2. **Glyph runs join into `TJ` arrays with exact kerns when representable.**
   A glyph continues the previous segment when
   `n = (adv·1000·size − 1000·Δ·upem) / (upem·size)` (thousandths of text
   space, `Δ` the origin gap in ticks) is a terminating decimal: zero joins
   the string, non-zero becomes a `TJ` number, otherwise the glyph gets its
   own `Tm`. The written operators are replayed exactly (`exact::glyph_positions`,
   reduced rationals) and every glyph is asserted to land on the envelope
   origin; output stays byte-deterministic.

**Measured outcome:** on this corpus the kern path never fires. The
pipeline sets body text at 12 TeX pt = 12,535,902 ticks (2 · 3 · 2089317),
and an adjustment in thousandths of *that* size terminates only when
`upem·size / gcd` has no prime factor but 2 and 5; the factor 3 makes every
non-zero gap non-terminating, and the same holds for the other sizes in use
(headings, scripts). Exactness was the requirement, so the streams stay at
one `Tm` per glyph; the test suite exercises the kern path with a
2,5-smooth size (12,500,000 ticks → `[(He)434.9196(H)] TJ`). Making the
pdfTeX shape reachable on real output would need the producer to choose
tick-exact sizes whose value has only factors 2 and 5, or a different
positioning contract (`Tc` in unscaled text space is exact for any tick
gap but is one operator per glyph, so it buys nothing).

| fixture | bytes before (whole Subr/String INDEXes, one Tm per glyph) | bytes after (pruned INDEXes, exact TJ where representable) | ratio | joined / kerned glyphs | categories or raster changed |
|---|---|---|---|---|---|
| 01-plain-paragraph | 29,547 | 9,595 | 32% | 0 / 0 | no |
| 02-wrapping-paragraph | 96,447 | 76,903 | 80% | 0 / 0 | no |
| 03-section-heading | 54,935 | 14,765 | 27% | 0 / 0 | no |
| 04-bold-emph | 104,089 | 17,232 | 17% | 3 / 0 | no |
| 05-unicode | 28,931 | 8,859 | 31% | 0 / 0 | no |
| 06-math-inline | 140,263 | 11,719 | 8% | 0 / 0 | no |
| 07-math-display | 139,697 | 11,513 | 8% | 1 / 0 | no |
| 08-two-page | 628,112 | 608,568 | 97% | 0 / 0 | no |
| 09-mixed-document | 203,293 | 32,523 | 16% | 2 / 0 | no |
| 10-unicode-paragraph | 62,869 | 47,876 | 76% | 0 / 0 | no |
| 11-nested-lists | 36,762 | 17,444 | 47% | 0 / 0 | no |
| 12-justified-paragraphs | 146,226 | 126,682 | 87% | 0 / 0 | no |
| 13-math-display-rich | 145,014 | 17,060 | 12% | 2 / 0 | no |
| 14-math-inline-dense | 155,485 | 27,491 | 18% | 1 / 0 | no |
| 15-three-page-sections | 652,110 | 611,989 | 94% | 0 / 0 | no |
| 16-heading-page-break | 371,875 | 332,089 | 89% | 0 / 0 | no |
| 17-apostrophes | 36,418 | 17,238 | 47% | 0 / 0 | no |
| 18-ligatures | 39,690 | 19,958 | 50% | 0 / 0 | no |
| **total** | 3,071,763 | 2,009,504 | 65% | | |

Categories (content, fonts, page geometry) and raster results are identical
to the first run on all 18 fixtures; the size reductions come entirely from
the font programs, which is why the text-heavy multi-page fixtures (08, 15,
16, 12), dominated by their content streams, change least.

## Producer tip d556519 and the math-reference fixture

Re-running the 18 fixtures with `flashtex-render` at `d556519` (which now
emits `fonts[].sha256` as SHA-256 of the raw font bytes, so `from-v2`
resolves every font by the contract's hash form and no deviation note is
produced) changed only the five math fixtures, all for the better on the
producer's side: 06 ink-only 162/170 → 38/51, 07 1396/1478 → 306/187,
09 3289/3335 → 147/199, 13 3388/4316 → 2289/2745, 14 5829/6178 →
2974/3242. Text-only fixtures are unchanged (01/04/05/17 still 0 ink-only
pixels). `from-v2` needed no change; the checked-in test envelope was
regenerated from this tip and the older hash form remains accepted and
reported (test).

rendering-core's `math-reference` fixture (`8b06436`: producer `65dbe7d`
display list exported through this crate's exact writer by rendering-core's
`pipeline_fonts_probe`, against the same pdflatex-lmodern reference as
corpus fixture 06; rendering-core measured Poppler text identical and 602
pixels differing at 144 dpi with `pdftoppm`) through `flashtex-pdf-exact
classify reference.pdf original.pdf`:

| measure | result |
|---|---|
| page count, MediaBox | same |
| content | `ContentOperators` (52 vs 133 operators: pdfTeX `TJ` with `11.9552 Tf`/`708.045 Td`, original `Tm` per glyph with `11.9551677703857421875 Tf`/`708.0448322296142578125 Tm`) |
| fonts | `FontProgram` + `FontMetadata` for `LMRoman12-Regular` (Type 1 subset 23,000 B vs CIDFontType0C 23,130 B, unpruned since the fixture predates subroutine pruning); `FontResources`: `LMMathItalic12/8`, `LMMathSymbols10` and pdfTeX's second `LMRoman12` dictionary only in the reference, `LatinModernMath-Regular` only in the original |
| CoreGraphics 144 dpi | 645 of 1,938,816 RGB pixels differ; ink-only 38 (reference) / 51 (original); registration shift 0 px |
| exit status | 3 (content operators and font programs not identical, as expected) |

The 645 CoreGraphics pixels against rendering-core's 602 Poppler pixels are
the two rasterisers' anti-aliasing; the binary-ink count of 38/51 says the
glyphs sit where pdfTeX's do and the only visible difference is Latin
Modern Math versus Computer Modern math glyph shapes.

## Type 1 subsetting, apples to apples

pdfTeX embeds Latin Modern as Type 1 subsets, so a `FontProgram` line
between pdfTeX's output and this crate's CFF subsets compares different
program formats. `crate::type1` now subsets Type 1 programs the way pdfTeX
does: only the charstrings the placed glyphs need (plus `.notdef` and `seac`
components), unused subroutines blanked to `return` so indices survive,
retained charstrings and subroutines byte-identical to the source, the
private dictionary re-encrypted deterministically, `Length1` unchanged,
`Length3 = 0`. `ExactFont::type1_subset` builds the simple font (Differences,
caller-supplied widths, descriptor from the clear text). The classifier
compares two embedded Type 1 programs glyph by glyph: for the pdflatex
oracle document in `tests/type1.rs` (`lmr12.pfb`, 21 glyphs) it reports
`Type 1 charstrings: 21 common glyph(s) identical, 0 differ … subroutines
used by the common glyphs identical` as `[same]` while the bytes differ
(pdfTeX: 23,421 B; ours: 26,981 B, 38 of 784 Subrs retained; pdfTeX also
trims the clear text). Widths remain a caller decision: Latin Modern's
charstrings compute widths with `div` (`4787 11 div` = 435.18…), which is
why pdfTeX takes them from the TFM; `Type1Font::advance_width` returns the
exact rational and the oracle test reuses pdfTeX's widths. Feeding Type 1
programs into the runtime-v1 or from-v2 routes is not wired (both need a
code → glyph-name source such as a TeX `.enc` file); the library and the
classifier verdict are what this step adds.

## What this does and does not establish

- Established: a real FlashTeX producer can drive the exact route end to end
  with original glyph ids, exact tick positions, embedded GID-preserving
  subsets, typed rules and searchable text, and the output is deterministic
  (test) and structurally valid (self-check on all 18).
- Established: for text-only fixtures the remaining visible gap to pdflatex
  is at the anti-aliasing level; the remaining measurable gaps are the
  pipeline's documented limitations (lists, `\left`/`\right`, some symbols,
  hyphenation) and its choice of math font.
- Not established: parity of operator streams or font programs with pdfTeX
  (different representation and program format by design), or anything
  outside these 18 fixtures, this font, this page size and CoreGraphics.

## Needs from other owners (not edited here)

- **render-pipeline**: the envelope's `fonts[].sha256` is font-engine's
  `content_sha256` = SHA-256(bytes ‖ face_index as big-endian u32), not
  SHA-256(bytes) as `docs/contracts/rendering-v2-proposal.md` describes and
  `crates/rendering-core`'s validator checks (`digest(bytes) == font.sha256`,
  "font digest mismatch"). `from-v2` accepts both and reports which matched;
  emitting SHA-256(bytes) would make the envelope validate as is.
- **render-pipeline**: `core14-afm` (Times, metric-only) carries no program;
  the exact route refuses runs in it. Either ship a redistributable Times
  program as a real resource or keep Times on the v1 route only.
- **rendering-core / schema**: `format: "opentype-cff"` (the pipeline's
  documented deviation from the schema's `static-truetype` const); `from-v2`
  accepts it and verifies the file really has CFF outlines.
- **rendering-core**: cluster ActualText is reduced to per-glyph ToUnicode
  here; a marked-content `/ActualText` contract would need `BDC`/`EMC` in the
  bounded operator set (not added; no conflicts were observed on the corpus).
- **this crate (mac-pdf)**: done in the follow-up below — subroutine and
  string INDEXes are pruned (LM Math 7-glyph subset 1,753 bytes) and `TJ`
  kerns are emitted when exactly representable, which on this producer's
  sizes is never (see below).

The rendering-core outline route (`crates/rendering-core/src/pdf_export.rs`,
paths only, no embedded fonts or searchable text) and this glyph-run route
are complementary: the outline route proves operator/geometry exactness for
paths; this route carries fonts and text.

## Reproduction

```sh
# producer (scratch archive, never in the product path of this crate)
git archive origin/agent/mac-render-pipeline/unified crates/render-pipeline | tar -x -C /tmp/rp
cargo build --release --manifest-path /tmp/rp/crates/render-pipeline/Cargo.toml --bin flashtex-render
printf '%s\n' '{"protocol_version":1,"id":"x","type":"compile","payload":{"project_id":"p","revision":1,"entry_path":"main.tex","documents":[{"path":"main.tex","text":"\\begin{document}Hello world.\\end{document}\n"}]}}' \
  | /tmp/rp/crates/render-pipeline/target/release/flashtex-render --secnumdepth 0 --v2 list.json > /dev/null
# adapter + comparison
cd crates/pdf && cargo build --release
target/release/flashtex-pdf-exact from-v2 list.json --out ours.pdf
target/release/flashtex-pdf-exact classify reference.pdf ours.pdf
```

`cargo test` (86 tests: 42 unit, 12 `tests/exact.rs`, 4 `tests/v2.rs`, 3
`tests/type1.rs`, 25 `tests/render.rs`) and `cargo clippy --all-targets` are
clean at the commit that updates this file. `tests/v2.rs` carries the unmodified `flashtex-render`
envelope for fixture 01 (`tests/fixtures/v2-plain-paragraph.json`) and
resolves Latin Modern by content hash from the installed TeX Live tree.
