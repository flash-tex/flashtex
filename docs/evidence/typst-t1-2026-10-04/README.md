# Typst T1 evidence (2026-10-04)

Lane TYPST-T0T1, phase T1 of DESIGN.md §15.10: `flashtex-typst-host` (`typst-host/`).
Machine: mac-m1max-a (M1 Max, 10 cores, 32 GB), shared with other agents' builds and
benchmarks. **Every timing here is non-reference**: the load average is recorded with
each run.

## Positions checker vs typst-pdf (T1's correctness gate)

Gate: 0 mismatches on the corpus and on every Typst test-suite snippet that compiles.

| Set | Compiled | Pages | Glyphs compared | Mismatches | Verdict |
|---|---|---|---|---|---|
| Corpus: `typst-host/tests/fixtures` (text, math, shapes, links, transformed text with bleed), through the host process | 5 / 5 | 8 | 1,848 | 0 | MET |
| Typst 0.15.1 `tests/suite` (tag `9dfd3a08`), typst-dev-assets `53c12796`: 3,684 snippets | 2,622 | 3,162 | 80,556 drawn (82,014 in the PDFs) | 0 glyphs, 0 boxes, 0 pages failed | MET |

What "mismatch" means: a glyph's origin (ORIGINS) or glyph matrix (MATRIX) not
bit-identical (f64 bits) to the checker's, or a GLYPH's sp position not the sp rounding of
that origin, or a page box not bit-identical; a page whose positions the host could not
derive counts as failed. The 1,458 PDF glyphs not drawn are those of runs the host flags
INCOMPLETE (gradient or tiling fills, spot colour) and the inline text of SVG images; the
checker places them independently from the frames.

- Method: `typst-host/examples/positions_suite.rs` runs each snippet through the host's
  own conversion (`convert::page` with `pdfpos`-derived positions, what a 3.3 client
  receives) and compares with `typst-host/tests/checker`, which reads the document's
  whole default export (tagged, compressed: `DONE.pdf`'s bytes) with its own object scan,
  inflate (`flate2`) and number reading. Raw summary: `positions-suite.json`.
- The 1,062 snippets that do not compile here are listed by reason in
  `positions-suite-not-compiled.json`: errors the suite tests on purpose, HTML and bundle
  targets, `@test` packages (refused until the package lock lands), the suite's native
  helpers. They are outside the gate's "every snippet that compiles".
- Run: 37.7 s for the whole suite at a load average of 104–117 (`uptime` before the run).
- Iterations that found real issues before the final run: image XObjects (an SVG or PDF
  image's text is the image's, not the page's) and SVG text drawn inline by krilla; both
  are handled and the page fails closed (INCOMPLETE) when a run is not where the PDF
  shows it.
