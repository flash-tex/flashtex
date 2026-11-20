# Unicode differential fuzzer (fuzz-unicode)

Differential fuzzing of FlashTeX's XeTeX engine (`crates/flashtex-xetex`)
against TeX Live 2026's `xetex` on generated documents. MIT, Python 3
standard library only (plus fontTools and qpdf for `--pdf`). TeX Live's
xetex is the oracle and only the oracle (DESIGN.md §13); nothing here is
in the product path. It is the Unicode counterpart of `tools/fuzz-parity`
(pdfTeX) and reuses `tools/xetex-lockstep` for running and comparing.

## Run

```sh
(cd crates/flashtex-xetex && CARGO_TARGET_DIR=/tmp/fx cargo build --release --locked)
python3 tools/fuzz-unicode/fuzz.py --candidate /tmp/fx/release/flashtex-xetex \
    --seeds 0:500 --workers 2 --out /tmp/fzu --timeout 60
python3 tools/fuzz-unicode/fuzz.py ... --latex          # xelatex + fontspec documents
python3 tools/fuzz-unicode/fuzz.py ... --pdf            # also the PDFs (python with fontTools)
python3 tools/fuzz-unicode/fuzz.py ... --narrow         # max_print_line 60, error_line 50: wrapping cuts UTF-8 output
python3 tools/fuzz-unicode/fuzz.py ... --area bidi      # every chunk from one area (gen.AREAS)
python3 tools/fuzz-unicode/show.py /tmp/fzu/log/17      # the first difference
python3 tools/fuzz-unicode/minimize.py --candidate BIN /tmp/fzu/log/17   # delta-debug
python3 tools/fuzz-unicode/gen.py 17 [--latex]          # print one document
```

- `--seeds START:COUNT`: each seed is one document; `gen.generate(seed)` is
  a pure function of the seed, so a seed number plus gen.py's revision is
  the whole test case. A grammar change renumbers the cases.
- `--workers` is capped at 2. On a shared Mac run it under `nice -n 19`.
- Each non-equal seed is written to `OUT/<class>/<seed>/` (`doc.tex`, both
  normalised logs, `result.json` with the first difference);
  `OUT/summary.json` groups seeds by signature. Exit 0 when every seed was
  equal or neutral.

About 0.7 s a document per engine on an M-series Mac: 300 seeds take
about four minutes with two workers.

## What runs

Plain documents run through `tools/xetex-lockstep/run.py`'s `run_one`:
both engines through a symlink named `xetex`, `-ini -etex -no-pdf`,
`-interaction=nonstopmode` **without** `-halt-on-error` (error recovery is
compared too), max_print_line 1000, error_line 254, `SOURCE_DATE_EPOCH=0`,
`FORCE_SOURCE_DATE=1`, `TZ=UTC`, each in its own directory holding the
picture files of `tools/xetex-lockstep/pictures`.

`--latex` documents run through `tools/xetex-lockstep/latex.py`: each
engine builds its own `xelatex.fmt` once (`OUT/fmt`, about a minute each),
then each document runs untraced and then traced to its box dumps.

## The replay axis (`--replay`)

`--replay` needs no oracle: "off" is the reference. Each `gen.py --latex`
document (60% of them load hyperref, whose every `\pdfstringdef` runs
`\pdfstringdefPreHook`, XeTeX's guarded intrinsic; 40% siunitx, which fills
it) runs through the candidate three times, `FLASHTEX_INTRINSICS=off`,
`on` and `verify` (with `FLASHTEX_INTRINSICS_VERIFY_FAIL=1` and
`FLASHTEX_INTRINSICS_STATS`), two untraced passes each so the `.aux` and
`.out` files are read back. A finding (class `replay`) is a log, exit
status or XDV of `on` or `verify` that differs from `off`, a verify exit 3
or a `verify.differing` count. The run prints how many calls were replayed
and checked, so a run that never reaches a replay is visible. The
candidate must be a build with XeTeX intrinsics (#2134, #2140, #2251, not
yet on main); it builds its own `xelatex.fmt`.

```sh
python3 tools/fuzz-unicode/fuzz.py --candidate <intrinsics build> --replay --seeds 0:100 --out /tmp/fzr
```

## What is compared (P-T1, and the XDV rule of docs/design/xetex/PLAN.md §3.5)

In order; the first difference names the class:

| class | difference |
|---|---|
| `candidate-crash` | the candidate died by a signal or panicked (exit 101) |
| `log` | the log after the lockstep normalisation (`compared_lines`) |
| `rc` | exit statuses |
| `xdv` | the XDV after `normalise_xdv` (preamble date, font paths) |
| `terminal` | the terminal output |
| `timeout` | one engine exceeded `--timeout` |
| `pdf` | with `--pdf`: tools/xetex-pdfparity/compare.py (structural, `--no-visual`) fails on the two PDFs (xdvipdfmx's and the candidate's own) |
| `latex-*` | with `--latex`: latex.py's verdict (log, XDV, exit status, panic) |

`--pdf` findings need a second look before they are filed: the comparison
is with xdvipdfmx, not TeX, and two kinds of difference are deliberate or
known. xdvipdfmx's glyphs drift up to 0.005 em from TeX's positions in
TrueType fonts (its `/W` widths are rounded; tools/xetex-pdfparity/README.md,
"xdvipdfmx's precision"), so a `glyph-position` just over the tolerance in
a large TrueType font (Charis SIL, Amiri) is xdvipdfmx's, not the writer's.
The writer marks vertical native fonts (`:vertical`) unsupported and draws
them upright (`glyph-matrix`, `src/out/page.rs`). A `\XeTeXglyph` past the
font's last glyph has no advance in TeX, but xdvipdfmx gives it a `/W`
width (the CFF default, e.g. 280) and the rest of its `TJ` drifts by that
much (`glyph-width` with `glyph-position`); the writer keeps TeX's
positions. A run where xdvipdfmx
itself fails ("driver return code") is not compared.

Neutral outcomes, not failures: `both-hang`, `both-signal` (the same signal
in both), and `oracle-crash`: xetex itself died by a signal (undefined
behaviour; kept under `OUT/oracle-crash/` for reference). TeX Live 2026's
xetex segfaults, for example, when `\XeTeXinterwordspaceshaping` is
positive and two native words of a font are separated by an explicit glue
before any space was typed in that font: it reads `width(font_glue[f])`
of a null pointer (#2391). The port then stops on a checked index (exit
101). The grammar usually types a space in each new font first
(`Gen.prime`) so this crash does not hide the rest of a document.

## Grammar (gen.py)

A document is a preamble and 4-12 independent chunks, every line balanced
on its own. Fonts are TeX Live's, named by file (`"[name.otf]"`), so no
system font lookup is involved: Latin Modern, TeX Gyre, Libertinus, Amiri
(Arabic), David CLM (Hebrew), Charis SIL, Fandol Song (CJK), Garuda and
Norasi (Thai), the variable Junicode VF and Pennstander VF, and the Latin
Modern, TeX Gyre and Libertinus math fonts. The preamble picks the
tracing switches, paragraph parameters, `\patterns` and `\hyphenation`,
2-5 fonts with random OpenType features, scripts, languages, `mapping`,
`letterspace`, `extend`, `slant`, `embolden` and `color`, sometimes a math
setup through `\Umathcode`/`\Udelcode`/`\Umathchardef`, and
`\XeTeXinterwordspaceshaping`, `\XeTeXdashbreakstate`,
`\XeTeXupwardsmode`, `\XeTeXuseglyphmetrics`, `\XeTeXinputnormalization`,
`\XeTeXtracingfonts` and `\XeTeXlinebreaklocale`. Chunks:

- paragraphs and boxes of mixed Latin, Hebrew, Arabic and digit words, with
  font changes inside words, bidi and joining controls, `\XeTeXglyph`,
  `\char`, `\Uchar`, kerns, penalties and discretionaries;
- bidi boxes (one RTL script, Latin and numbers, controls, TeX--XeT);
- `\XeTeXinterchartoks` with `\XeTeXcharclass` (classes 0, 1-4, 4095);
- font queries (`\XeTeXcountglyphs`, the `\XeTeXOT...` queries,
  `\XeTeXglyphindex`, `\XeTeXglyphname`, `\XeTeXglyphbounds`, ...);
- ligatures and kerns across font changes, `\/`, `\noboundary`;
- hyphenation of native words in narrow boxes, `\language`;
- OpenType math: scripts, fractions, radicals, accents, delimiters,
  `\Umathchar`, `\Uoverdelimiter`, `\Uroot`, displays;
- `\XeTeXpicfile`/`\XeTeXpdffile` with every size keyword;
- a new font mid-document, `\fontdimen` changes;
- ICU line breaking under `\XeTeXlinebreaklocale` (Thai, CJK);
- `\showlists` inside a box.

`gen.py --latex` writes `xelatex` documents with fontspec (main font by
name or file, Hebrew and Arabic families, feature changes,
`\XeTeXinterwordspaceshaping`) and sometimes unicode-math.

## Tests

```sh
python3 -m unittest discover -s tools/fuzz-unicode -p 'test_*.py'
FUZZ_UNICODE_ENGINE=/tmp/fx/release/flashtex-xetex python3 -m unittest discover -s tools/fuzz-unicode -p 'test_*.py'
```

The pure-logic tests (determinism, balanced lines, ddmin) always run; with
`FUZZ_UNICODE_ENGINE` and xetex on PATH, fixed seeds must be clean.

## Findings

| issue | what | fix |
|---|---|---|
| #2329 | `\XeTeXinputnormalization` ignored: `input_line` never called the TECkit normalizer | #2340 (crates/flashtex-xetex/src/system.rs) |
| #2409 | PDF writer: `embolden=` stroke width scaled by the font size twice (only 10 pt right) | #2410 (crates/flashtex-xetex/src/out/page.rs) |
| #2432 | Thai `\XeTeXlinebreaklocale` breaks of non-dictionary text (fragments, merged words) differ from ICU's ThaiBreakEngine (documented gap: ICU4X segmenter) | open |
| #2391 | not a difference: xetex segfaults and the port panics on `width(font_glue[f])` of null under `\XeTeXinterwordspaceshaping` | open (decision: keep the crash or deviate) |
| #2382 | `\XeTeXinputencoding` with an ICU converter name (`latin1`, `cp1252`, ...) reads raw bytes (a documented gap: no ICU converters) | open; the grammar uses XeTeX's built-in names only |

Measured 2026-11-09 on mac-m5pro-kabir. The oracle was MacTeX 2026: xetex 0.999998, ICU 78.2 and xdvipdfmx. The candidate was main ca202f597 with #2340 and #2410, except that the replay runs used #2251's head.

| run | documents | differences |
|---|---|---|
| plain | 5,600 | 3, all #2432 |
| plain, `--narrow` | 1,600 | 1, #2432 |
| dense areas | 18 runs, 1,640 | 76 in the first encoding run, all #2382; none after the grammar change |
| LaTeX (fontspec, hyperref, siunitx, unicode-math) | 760 | 0 |
| PDF | 366 | #2409, and otherwise only the xdvipdfmx and writer kinds above |
| replay | 540 | 0, in 2,015 replayed calls, each also verified |

The "differences" column leaves out the neutral outcomes: xetex segfaulting (about 3% of plain documents, mostly #2391), both engines hanging, and both dying by the same signal.
