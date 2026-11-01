# PDF searchable text: policy and contract (GH48)

Status: proposal from lane `mac-pdf-3` (2026-09-12), for the Commander, the
producer/Text owner (`crates/rendering-core`) and the PDF owner (`crates/pdf`).
Answers GH48's request for "an explicit searchable text policy/contract and
bounded occurrence-level support if required". Every number below was
measured on mac-m1max-a with the commands in "Reproduction" unless marked
*unmeasured*.

## What the exact route promises

`flashtex-pdf-exact from-v2` writes, for every glyph of the display list:

1. **Its original glyph id** in a GID-preserving CID subset, at **its
   envelope origin exactly** (`Tm`, or a `TJ` adjustment that replays to the
   same tick; `exact::glyph_positions` is tested on every fixture).
2. **A per-glyph ToUnicode entry** = the cluster text of the display list
   (`ffi` → `ffi`, `fi` → `fi`, `é` → `é`). A glyph seen with two different
   cluster texts keeps the first and the report says so.
3. **`/W` widths equal to the producer's own kern-free advances** where they
   differ from hmtx, so that the pen after a glyph lands where the producer's
   pen landed (mac-pdf-2, `4893f3e7`).

**Word boundaries are geometry.** No space glyph, no `Tw`, no ActualText: an
extractor decides a word break from the gap between the pen after one glyph
and the origin of the next, and the export guarantees that gap is the
producer's glue to the tick. The contract, in thousandths of the font size
of the glyph before the gap:

| gap `g` | promise |
|---|---|
| `g >= 150` (`v2::WORD_GAP_EM`) | a word boundary in PDFKit (Preview, the app, `PDFDocument.findString`), Spotlight (`mdimport`), and Poppler `pdftotext -raw` (its `minWordSpacing` constant; *unmeasured here*, Poppler is not installed on this Mac). A Computer Modern word space is 326–333; shrunk to TeX's minimum it is still ≥ 217. |
| `30 <= g < 150` | **ambiguous** — extractor-dependent; the export report counts these (`ambiguous_gaps`) and names the first eight. Measured: PDFKit breaks from 100, Ghostscript `txtwrite` from 250, Poppler `-raw` from 150 (constant), Poppler default/`-layout` from 30 when the line has a word longer than one glyph. In practice this is TeX's math italic correction (99 after math `f`). |
| `g < 30` (`v2::CHAR_GAP_EM`) | never a boundary in any measured extractor (kerns, TFM/hmtx rounding). |

The threshold is expressed in em, not "one word space of the current font",
because the extractors do not know the font's word space; 150 is the lowest
value that every promised extractor honours and that every shrunk Computer
Modern space clears.

## What it does not promise

- **No ActualText / marked content**: outside the bounded operator set; the
  rendering guard refuses mappings that would need it. Consequence: a glyph
  reused with different cluster texts extracts as the first text.
- **No ligature decomposition beyond ToUnicode**: `ffi` is one code with a
  three-character ToUnicode; extractors that ignore multi-character
  `bfchar` destinations (Ghostscript 10.08 `txtwrite` prints `Ï`) are theirs
  to fix. PDFKit and Spotlight decode it.
- **No word boundary for Poppler default/`-layout` on a line made only of
  one-glyph words** (GH48's `\text{a b}` alone on its line). Poppler treats
  such a line as letter-spaced text: with all words of length 1 it sets the
  break threshold to `min(1.3 × smallest gap, 0.4 em)` and merges `a`+`b`
  (gap 0.326 em < 0.4 em). The published evidence (647c50c5) shows the
  *same* `ab` for the f261 output that carried an explicit space glyph, and
  the same code path explains it (a space glyph ends a word but is not kept;
  the merged line still gets no `spaceAfter`). Nothing geometry-preserving
  changes this; only moving `b` ≥ 0.4 em away would, which is forbidden.
  **Acceptance mode for Poppler is `-raw`** (as the Commander's triage
  recommended), plus PDFKit and Spotlight, which are what the product ships
  with.
- **No spaces inferred from source**, no second parser/writer, no glyph moved.

## Why the current mechanism was chosen (evaluation)

Three geometry-preserving ways of encoding a word boundary were considered:

| mechanism | PDFKit / Spotlight | Ghostscript | Poppler `-raw` | Poppler default | cost |
|---|---|---|---|---|---|
| **gap only** (`Tm` or `TJ` adjustment; what is written today) | `a b` at ≥ 100 (measured) | `a b` at ≥ 250 (measured) | `a b` at ≥ 150 (source constant) | `ab` on one-glyph lines; `ab cd` on normal lines (from 30) | none; already tested |
| **space glyph** (font's own `space` GID at the pen, `/W` = its width, next glyph on its origin) | would add a ` ` code; PDFKit already breaks | would add a ` ` code | already breaks | still `ab` on one-glyph lines (evidence 647c50c5 `space-before`) | an extra glyph not in the display list (breaks the replay check "every glyph is the list's"), a subset entry, and fonts without a space glyph need a fallback |
| **`Tw`** | not applicable | not applicable | not applicable | not applicable | `Tw` applies only to single-byte code 32; all exact-route fonts are Identity-H CID fonts |

`TJ` vs `Tm` for the gap is not an extractor-visible choice: both replay to
the same origin, and the export already picks `TJ` whenever the adjustment
is an exact decimal. So the chosen mechanism is the existing one, made
explicit and measured: the crate now reports `word_gaps` and
`ambiguous_gaps` (stderr `note: searchable text: …`) so an export's
extraction risk is visible without running an extractor, and tests pin the
exact PDFKit strings.

## Bounded occurrence-level rule (request to the producer)

The export will emit an explicit space **only where the display list marks
one**, never by inferring it from glue size or source. The exact request to
the producer/Text owner, if the Commander wants Poppler default/`-layout` to
read `\text{a b}` as two words on a one-glyph line:

> Add an optional cluster kind `"space"` to `glyph_run.clusters[]`
> (`{"kind":"space","text_start_byte":i,"text_end_byte":i+1,"x":ticks,"advance_x":ticks}`
> with no glyph), emitted only where the producer itself typeset an
> inter-word glue from a source space inside one logical run. Runs split at
> a `\text{}` / math boundary are unaffected.

With that marker `from-v2` would write the font's own `space` glyph (no
ink; raster unchanged) at the marked `x` with `/W` = the marked advance and
ToUnicode ` `, and refuse (not approximate) when the font has no `space`
glyph. Note this still does **not** fix Poppler default/`-layout` for the
one-glyph line (see above); it only makes the boundary explicit for
extractors that keep space codes. The measurements above suggest it is not
worth the replay-check exception today; this proposal does not implement it.

## Measurements (2026-09-12, mac-m1max-a, load 34–120)

Producer: bundled `flashtex-render` (FlashTeX.app 12:00) for text cases; the
published GH48 display list (647c50c5 `space/display.json`) for
`\text{a b}`. Exporter: `crates/pdf` at this lane's tip. Extractors: PDFKit
(`PDFPage.string`, macOS 26), `mdimport -t -d3` (Spotlight),
Ghostscript 10.08.0 `txtwrite`. `pdftotext` is not installed; Poppler
values come from 647c50c5's published `extracted-*.txt` (26.01.0) and from
`TextOutputDev.cc` constants.

| fixture | PDFKit / Spotlight | Ghostscript | Poppler (published) |
|---|---|---|---|
| `\text{a b}` (GH48 list) | `a b` | `a b` | default/`-layout` `ab`, `-raw` `a b` |
| text `a b` | `a b` | `a b` | — |
| `ab cd` | `ab cd` | `ab cd` | — |
| `office ffi` | `office ffi` | `ofÏce fÏ` | — |
| `AVAV Wednesday` | `AVAV Wednesday` | `AVAV Wednesday` | — |
| `$\forall x$` (source-text fallback) | `\f orallx` | `\forallx` | — |
| mixed line | `The AV office fixed a b: \f orallx, f (x) and ffi.` | `The AV ofÏce fixed a b: \forallx,f(x) and fÏ.` | — |

Gap sweep on `a b` (gap set to N/1000 em, everything else fixed): PDFKit
`ab` ≤ 80, `a b` ≥ 100; Ghostscript `ab` ≤ 220, `a b` ≥ 250.

Report census on the fixtures: `\text{a b}` 1 word gap, 0 ambiguous; mixed
line 9 word gaps, 2 ambiguous (99/1000 em after math `f`, before `o` and
`(`) — exactly the two places PDFKit and Ghostscript disagree.

PDF bytes from the crate before and after this lane's change are identical
for all seven fixtures (`cmp`), so the 144 dpi raster is unchanged by
construction; the change is report-only.

## Extractor disagreements that remain

- Poppler default/`-layout` on lines consisting only of one-glyph words
  (`ab`): Poppler's letter-spacing heuristic; not fixable without moving
  glyphs or a ≥ 0.4 em gap. Use `-raw`.
- Math italic correction (99/1000 em) after italic `f`, `x`, …: PDFKit and
  Spotlight read a space, Ghostscript and Poppler `-raw` do not. pdfTeX
  places the same kern. The export names each occurrence; a producer-side
  fix would be to fold the correction into the glyph's `advance_x` for the
  `/W` width, which is the producer's call (it would then be a
  "kern-free advance" that differs by glyph context).
- Ghostscript `txtwrite` prints `Ï` for the `ffi`/`fi` ligature codes
  (multi-character ToUnicode); PDFKit and Spotlight decode them.
- `$\forall x$` and `\text{}` are typeset as their source text by the
  bundled producer build (`\forall is not supported in math mode`); the
  math symbol case is therefore covered only by the published GH48 list and
  the `x`/`f` italic glyphs of LatinModernMath, not by a real `∀`.

## Reproduction

```sh
cargo build --release --manifest-path crates/pdf/Cargo.toml
cargo test  --release --manifest-path crates/pdf/Cargo.toml --test v2   # 6 tests
crates/pdf/target/release/flashtex-pdf-exact from-v2 crates/pdf/tests/fixtures/v2-text-a-b.json \
  --out /tmp/ab.pdf --font-dir apps/mac/Fonts      # stderr: "note: searchable text: 1 word gap(s) …"
cd apps/mac && FLASHTEX_PDF_EXACT=$PWD/../../crates/pdf/target/release/flashtex-pdf-exact \
  swift test --filter SearchableTextTests            # 3 tests, PDFKit exact strings
gs -q -dNOPAUSE -dBATCH -sDEVICE=txtwrite -sOutputFile=- /tmp/ab.pdf
mdimport -t -d3 /tmp/ab.pdf | grep kMDItemTextContent
```
