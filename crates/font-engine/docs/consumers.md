# One metric source for compiler layout, native preview and PDF (FT-018 rev 3)

Measured on mac-m1max-a, macOS 26.3.1, 2026-09-12, with the pinned Latin
Modern Roman 10 (`fonts/manifest.json`, sha256 `1aa18cfe…6852`) at 12 pt,
text taken from `tests/visual-corpus/fixtures/{kerning-ligatures,font-faces,
paragraph-linebreaks}.tex` (TeX input ligatures resolved to Unicode). Every
number below is produced by `cargo run --example corpus_evidence -- <dir>`
and `swift tools/preview_positions_check.swift <dir>/face_metrics.json`.

## Which consumer reads which source today

| Consumer | Source it can read now | Adapter | Status on main |
| --- | --- | --- | --- |
| Compiler / paragraph layout (`crates/paragraph-layout`) | `Face` through `adapters::paragraph::FaceMetrics` (`FontMetricsSource`) or `adapters::paragraph::glyph_run` (`GlyphRun::from_shaped`) | shipped | `crates/compiler` still measures with its own Times-Roman AFM table (`metrics.rs`); paragraph-layout's `core14` adapter is the interim source. Adopting `glyph_run` is the compiler owner's change. |
| Native preview (Swift) | `adapters::preview::face_metrics_json` — FontId, PostScript name, upem, per-gid advances, clusters → source ranges, pen-relative glyph positions in pt | shipped (JSON contract) + `tools/preview_positions_check.swift` proves CoreText draws those gids/positions identically | The Mac shell still shapes with SwiftUI `Text`; switching it to `CTFontDrawGlyphs` at the exported positions is the shell owner's change. |
| PDF (`crates/pdf`) | `adapters::pdf::to_pdf_embedded_subset` → `flashtex_pdf::embed::EmbeddedSubset` (subset or bare CFF program, glyph map, `/W`, descriptor) | shipped | The writer's `chars: char → CID` map cannot address ligature/mark clusters; `PdfFontProgram::to_unicode_cmap` (multi-char) and `Cluster::text` (ActualText) are exposed for it. `examples/pdf_roundtrip.rs` shows the cluster-aware content stream; PDFKit extracts the original Unicode from it (rev 2 evidence). |
| Math layout (`crates/math-layout`) | `adapters::math::OpenTypeMathFace` (`MathFontMetrics`) from `latinmodern-math.otf`'s `MATH` table | shipped | Parameters exact (axis 2.5 pt, rule 0.4 pt, num1 6.77 pt, … at 10 pt); glyph heights/depths for CFF faces are estimates (flagged by `heights_are_approximate()`); `MathVariants` not parsed, so no stretchy sizes yet. |

## Corpus widths per route (12 pt, Latin Modern Roman 10)

Δ = route − engine, in points. "paragraph (char)" is paragraph-layout's own
`shape_run` fed by `FaceMetrics`; "paragraph (shaped)" is `glyph_run`
(engine clusters into `GlyphRun::from_shaped`); "pdf" is the width a PDF
reader advances by from `/W` (1/1000 em rounding) plus the kerns carried as
`TJ` adjustments; "compiler" is the Times-Roman AFM table the compiler
currently uses (identical values to this crate's Core 14 table, asserted in
tests) — a different font, so its Δ is the size of the gap until the compiler
adopts the adapter, not a measurement error.

| fixture | text | engine pt | paragraph (char) Δ | paragraph (shaped) Δ | pdf /W+TJ Δ | compiler Times-Roman table pt (Δ) | ligatures | kern src |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| kerning-ligatures | AVATAR WA To Ta Yo VA ff fi fl ffi ffl office affinity | 278.4600 | +0.3360 | +0.0000 | +0.0000 | 272.5920 (−5.868) | 10 | Gpos |
| kerning-ligatures | AVATAR AVATAR ToToTo office office | 207.9720 | +0.0000 | +0.0000 | +0.0000 | 207.9480 (−0.024) | 4 | Gpos |
| kerning-ligatures | (AV) [To] “office” — – - 0123456789 | 190.2960 | +0.0000 | +0.0000 | +0.0000 | 185.2800 (−5.016) | 2 | Gpos |
| font-faces | Roman: AVATAR office 0123456789. | 191.6520 | +0.0000 | +0.0000 | +0.0000 | 187.9800 (−3.672) | 2 | Gpos |
| paragraph-linebreaks | A student writes a careful explanation of an equation. | 284.2200 | +0.0000 | +0.0000 | +0.0000 | 256.9440 (−27.276) | 0 | Gpos |
| paragraph-linebreaks | Repeated words expose changes in font widths: minimum maximum minimum maximum. | 470.1000 | +0.0000 | +0.0000 | +0.0000 | 433.6680 (−36.432) | 0 | Gpos |
| paragraph-linebreaks | Hyphenation illustrates international collaboration and computational mathematics. | 443.0280 | +0.0000 | +0.0000 | +0.0000 | 400.2960 (−42.732) | 0 | Gpos |
| paragraph-linebreaks | The same font is constrained to a narrower measure. | 276.3960 | +0.0000 | +0.0000 | +0.0000 | 250.9320 (−25.464) | 0 | Gpos |

Reading:

* **paragraph (shaped) and pdf: 0.0000 pt on every line.** The engine's
  clusters, glyph ids and kerned advances pass through `GlyphRun::from_shaped`
  unchanged, and `/W` + `TJ` reproduce them exactly (LM is 1000/em, so `/W`
  rounding is lossless; for a 2048/em TrueType face `/W` rounding is bounded
  by 0.5/1000 em per glyph = 0.006 pt at 12 pt).
* **paragraph (char): +0.336 pt on the one line containing `ffi`/`ffl`.**
  The `FontMetricsSource::ligature` contract is char→char; Latin Modern forms
  `ffi` as `f`+`f`→`ff` then `ff`+`i`→`ffi` across two GSUB lookups, which the
  char route cannot express, so it measures `ff`+`i` (861 units) where the
  engine has the `ffi` glyph (833). Everywhere else the two routes agree
  exactly. Consumers that want ligature-exact widths use `glyph_run`.
* **compiler: −0.024 … −42.7 pt.** Different font (Adobe Times AFM vs Latin
  Modern), not a measurement discrepancy: the compiler has not adopted the
  adapter yet. On text where Times and LM happen to agree (`AVATAR AVATAR
  ToToTo office office`) the gap is 0.024 pt; on running text LM Roman is
  ~9–10 % wider than Times, which is exactly why the compiler must switch to
  the shared source before line breaks can match the reference.

## Native preview: engine positions vs CoreText, same file

`tools/preview_positions_check.swift` draws the exported gids at the exported
`x_pt`/`y_pt` with `CTFontDrawGlyphs` and diffs against `CTLineDraw` of the
same text:

| run | engine width pt | CoreText width pt | Δ pt | ink A | ink B | differing px (8×) |
| --- | --- | --- | --- | --- | --- | --- |
| AVATAR WA To Ta Yo VA ff fi fl ffi ffl o… | 278.4600 | 278.4600 | −0.0000 | 29261 | 29261 | 0 |
| AVATAR AVATAR ToToTo office office | 207.9720 | 207.9720 | +0.0000 | 23328 | 23328 | 0 |
| (AV) [To] “office” — – - 0123456789 | 190.2960 | 190.2960 | +0.0000 | 17399 | 17399 | 0 |
| Roman: AVATAR office 0123456789. | 191.6520 | 191.6520 | −0.0000 | 21873 | 21873 | 0 |
| A student writes a careful explanation o… | 284.2200 | 284.2200 | −0.0000 | 27098 | 27098 | 0 |
| Repeated words expose changes in font wi… | 470.1000 | 470.1000 | +0.0000 | 47052 | 47052 | 0 |
| Hyphenation illustrates international co… | 443.0280 | 443.0280 | −0.0000 | 45957 | 45957 | 0 |
| The same font is constrained to a narrow… | 276.3960 | 276.3960 | +0.0000 | 25690 | 25690 | 0 |

Largest |Δ| 0.0000 pt; zero differing pixels on all eight runs. A preview
that draws from `face_metrics.json` is therefore indistinguishable from
CoreText's own shaping of Latin Modern for this corpus, while using the
engine's numbers (so PDF and preview cannot drift).

## What remains different, and why

| Gap | Owner | Size |
| --- | --- | --- |
| Compiler measures with the Times AFM table instead of the shared face | compiler | up to −42.7 pt per corpus line (font choice) |
| Preview shapes with SwiftUI `Text` instead of drawing exported gids | Mac shell | unmeasured until switched; the check above bounds it at 0 once switched |
| PDF writer's `chars` map drops ligature/mark clusters | pdf | ligature glyphs unaddressable from `chars`; use cluster CIDs + ActualText (shown in `examples/pdf_roundtrip.rs`) |
| Char-route ligature contract (paragraph-layout) cannot express multi-step GSUB ligatures | paragraph-layout / this crate | +0.336 pt on the corpus line with `ffi`+`ffl`; 0 elsewhere |
| `\kern0pt` suppression line of kerning-ligatures | this crate | the engine has no per-pair kern/ligature suppression input yet; the line is not measured |
| Math glyph heights/depths for CFF, stretchy delimiters (`MathVariants`) | this crate | estimates flagged; no variants |
| Unsupported lookup types in a requested feature | — | explicit `Error::UnsupportedFeature` (default), or a note when `fail_on_unsupported_lookups` is off |

## Reproduce

```sh
cd crates/font-engine
cargo run --example corpus_evidence -- /tmp/fe            # table 1 + /tmp/fe/face_metrics.json
swift tools/preview_positions_check.swift /tmp/fe/face_metrics.json /tmp/fe/run   # table 2 (+ PNGs)
```
