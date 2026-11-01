# `third_party/icu`

Two parts of TeX Live 2026's ICU, **unmodified**, for the XeTeX port
(`crates/flashtex-xetex`, `docs/design/xetex/PLAN.md` §3.1). Neither is
compiled or linked:

- **The line-break rules** (`brkitr/`): the compiled rule-based break
  iterator tables (`.brk`) that `ubrk_open(UBRK_LINE, locale)` loads, for
  `\XeTeXlinebreaklocale` (XeTeX_ext.c's `linebreakstart`,
  `crates/flashtex-xetex/src/native/linebreak.rs`, which runs them as ICU's
  `RuleBasedBreakIterator::handleNext` does). ICU's break-iterator code is
  not vendored, and the Southeast Asian dictionaries are ICU4X's
  (`icu_segmenter`, see linebreak.rs).
- **The bidirectional algorithm's sources** (`icu-src/`): the source of
  record for the Rust port in `crates/flashtex-xetex/src/native/ubidi/`.
  XeTeX's `measure_native_node` (`XeTeX_ext.c`) splits each native word
  into direction runs with ICU's `ubidi_setPara`, `ubidi_getDirection`,
  `ubidi_countRuns` and `ubidi_getVisualRun`. The paragraph level comes
  from the font's script (`getDefaultDirection`,
  `XeTeXLayoutInterface.cpp`), and each run is shaped with HarfBuzz in its
  direction. The port follows these files routine by routine. Its property
  data (`props_data.rs`) is generated from `ubidi_props_data.h` by
  `gen_props_data.py` and checked against it by the crate's tests.

## Pin

| | |
|---|---|
| Upstream | TeX Live source tree, GitHub mirror <https://github.com/TeX-Live/texlive-source> |
| Tag | `tags/texlive-2026.1` ("texlive-2026.1 tag based on r78399") |
| Commit | `6a300188053b8f2ded89dbd52293732a706b9c0e` (the same pin as `third_party/xetex`, `third_party/harfbuzz`, `third_party/teckit`) |
| Path | `libs/icu/icu-src/source/data/in/icudt78l.dat` (sha256 `d7da8e2e31312a3cb574d1a0ebd7b187daa988e15a9b70b78eb68b213a590608`), items `brkitr/line*.brk`; `libs/icu/icu-src/source/common/` (the bidi files below); `libs/icu/version.ac` |
| Version | 78.2 (`libs/icu/version.ac`; `xetex --version`: "Compiled with ICU version 78.2"; `U_ICU_VERSION` in `icu-src/source/common/unicode/uvernum.h`), Unicode 17 bidi data (`ubidi_props_dataVersion` 0x11) |
| Fetched | 2026-10-12 |

`crates/flashtex-xetex/tests/pinned_libs.rs` fails if this table pins
another version, if a file differs from `SHA256SUMS`, or if a file of
`brkitr/` or `icu-src/` is missing from it:

```sh
cd third_party/icu && shasum -a 256 -c SHA256SUMS
```

## The line-break tables

Extracted with ICU's `icupkg -x brkitr/line.brk -d out icudt78l.dat` (and
likewise each file); the six files are byte-identical to the same items of
ICU's own `icu4c-78.2-data-bin-l.zip`. TeX Live's data holds no LSTM models
(`icupkg -l` lists none), so ICU breaks Thai, Lao, Khmer and Myanmar by
dictionary.

| file | ICU rules (`icu4c/source/data/brkitr/rules/`) | used for |
|---|---|---|
| `line.brk` | `line.txt` (CSS strict) | every locale but `zh`, `ja`, `ko` |
| `line_cj.brk` | `line_cj.txt` | `zh`; `ja`/`ko` with `@lb=strict` |
| `line_normal.brk` | `line_normal.txt` | `ja`, `ko`; others with `@lb=normal` |
| `line_normal_cj.brk` | `line_normal_cj.txt` | `zh`, `ja`, `ko` with `@lb=normal` |
| `line_loose.brk` | `line_loose.txt` | others with `@lb=loose` |
| `line_loose_cj.brk` | `line_loose_cj.txt` | `zh`, `ja`, `ko` with `@lb=loose` |

## The bidi sources

`icu-src/source/common/` holds:

| file | contents |
|---|---|
| `ubidi.cpp` | `ubidi_setPara` and the algorithm: paragraph levels P2–P3, explicit levels X1–X10, paired brackets N0, weak and neutral types W1–N2 by state tables, implicit levels I1–I2, L1 |
| `ubidiln.cpp` | the runs and their visual order (L2) |
| `ubidiimp.h` | the algorithm's internal definitions |
| `ubidi_props.cpp`, `ubidi_props.h`, generated `ubidi_props_data.h` | the bidi class, paired-bracket and mirroring properties, as a `UTrie2` |
| `utrie2.h` | the trie lookup, `UTRIE2_GET16` |
| `unicode/ubidi.h` | the API and its constants |
| `unicode/uvernum.h` | the version |

`version.ac` is TeX Live's version record. The rest of ICU's code is left
out.

## Licence

Unicode License v3 (`LICENSE`, ICU 78.2's), with the ICU licence for code
from before 2016. The port in `crates/flashtex-xetex` reproduces both
notices in that crate's `LICENSE`, as they require. It is a permissive
licence, and may be combined with the crate's GPL-2.0-or-later.
