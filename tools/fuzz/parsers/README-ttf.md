# TrueType / OpenType fuzzer (`tools/fuzz/parsers/ttf.py`)

The OpenType entry of DESIGN.md §8's T6 parser fuzzers. It drives the
engine's port of pdfTeX's `writettf.c`
(`crates/flashtex-engine/src/pdftex/writettf.rs`): TrueType subsetting,
whole-font TrueType copying and OpenType (CFF) embedding.

Seeds: a small synthetic TrueType font built by `synthetic_ttf()` (always
present, so the fuzzer needs nothing from the TeX tree to have a seed),
plus real fonts resolved at run time with
`kpsewhich emo-lingchi.ttf DejaVuSans.ttf lmroman10-regular.otf
texgyretermes-regular.otf` (missing ones are skipped; nothing is copied
into the repo). The synthetic font has 26 glyphs named as `8r.enc` names
them, a `post` format 2 table, a format 4 `cmap`, long `loca`, a composite
`fi`, and checksummed tables; pdfTeX embeds it unmutated in both TrueType
modes.

Each iteration mutates one seed, writes it as `fuzz.ttf` or `fuzz.otf`
beside `fuzz.tfm` (a copy of `cmr10.tfm`) and `8r.enc` (both from
`kpsewhich`), and runs

```sh
BIN -cnf-line=shell_escape=f -fmt=pdftex -interaction=nonstopmode job.tex
```

with `FLASHTEX_POOL`, `FLASHTEX_FORMATS` and `SOURCE_DATE_EPOCH=0`, where
`job.tex` sets `Hello, World! 0123456789 fi AV` in the fuzz font under one
of three map lines, drawn per iteration:

| Mode | Map line | Path |
|---|---|---|
| `ttf-subset` | `+fuzz FuzzFont <8r.enc <fuzz.ttf` | `writettf`, subset by glyph name through `post` |
| `ttf-whole` | `+fuzz FuzzFont <<fuzz.ttf` | `writettf`, whole font copied table by table |
| `otf-whole` | `+fuzz FuzzFont <<fuzz.otf` | `writeotf` (pdfTeX refuses to subset OpenType) |

```sh
python3 tools/fuzz/parsers/ttf.py --candidate BIN --iterations N \
  --seed S --out DIR --timeout SEC
```

Classes: `ok` (exit 0), `font-rejected` (nonzero exit and the log names
the fuzz font in a pdfTeX error, `(file fuzz.ttf)` / `(file fuzz.otf)`),
`graceful-error` (any other nonzero exit without a crash), `crash`
(signal or exit 101; the return code alone decides), `hang` (over
`--timeout`), `output-flood` (killed by the shared output cap).

Mutations: byte flips, truncation, chunk deletion/duplication, and sfnt
structure-aware edits on top of a table-directory parse:

- directory: `numTables` set below, past or far past the real count; one
  record's offset or length pointed past the end, at another table or
  just short of it; one tag renamed to another table's (a duplicate) or
  to `zzzz` (a missing required table);
- `head`: `indexToLocFormat` (0, 1, 2, -1), `unitsPerEm` 0, magic, bbox;
- `maxp.numGlyphs`: 0, 1, one past or far past the `loca` table;
- `hhea.numberOfHMetrics`: 0, past `numGlyphs` or past `hmtx`;
- `loca`: one entry past `glyf`, before its predecessor, or odd;
- `glyf`: contour count, an `endPtsOfContours` entry, the instruction
  length; or the glyph rewritten as a composite that references itself,
  the next glyph, a glyph past `numGlyphs`, or that never clears
  `MORE_COMPONENTS` before the glyph's end;
- `post`: version, glyph count, a name index past the names, a Pascal
  string length past the table;
- `cmap`: subtable count or offset, a subtable length, format 4
  `segCountX2`;
- `name`: record count, string storage offset, one string's length or
  offset;
- `CFF ` (OpenType seeds): bytes of its header and first INDEX.

Crash/hang/flood inputs are stored as `OUT/<class>/<sha256-prefix>.ttf`
(or `.otf`) with a `.json` sidecar (`seed`, `origin`, `mode`, `maplines`,
`mutation`, `returncode`, `last_line`, `panic_location`, `signature`),
deduped by panic location or signal. All draws go through one
`random.Random(seed)`: the same seed reproduces the same run. `nightly.py`
runs it as fuzzer `ttf` (seed offset 700000).

Tests (`python3 tools/fuzz/parsers/test_ttf.py`): the synthetic font's
directory, checksums, `loca`, composite and `post` names; every mutation
changes the bytes and each structure-aware op shows up; fake shell-script
candidates (a marker panic, an always-ok exit, a font reject, a sleeper)
for determinism, class counts, the stored artifact and its dedupe.

## First findings (2026-10-02)

Against main `d5ab24e68`, 400 iterations at seed 1 found one panic,
`writettf.rs:1105` (`index out of bounds`): with `maxp.numGlyphs` 0,
`glyph_index` and `glyph_tab` are shorter than the two glyphs every subset
starts with (`.notdef`, `.null`). Review of the fix (#1383) found the same
class one step further: with `numGlyphs` 1 the engine did not panic but wrote
a PDF with a broken subset, because `write_glyf` reads the entry after each
glyph's (`glyph_tab[id + 1]` in C). pdfTeX 1.40.29 reads past the table in
both cases (undefined behaviour) and ends in `unexpected EOF` with no PDF
(TeX Live 2023's pdfTeX crashes). The engine now ends in a pdfTeX error for
any glyph id at or past `numGlyphs`
(`crates/flashtex-engine/tests/ttf_malformed.rs`: 0 and 1 fail with exit 1
and an error naming the font, 2 and 3 embed).
