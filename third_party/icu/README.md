# `third_party/icu`

Parts of TeX Live 2026's ICU, **unmodified**, for the XeTeX port
(`crates/flashtex-xetex`, `docs/design/xetex/PLAN.md` §3.1). None is
compiled or linked (the converter tables are a fourth part, see "The
converters" below):

- **The line-break rules** (`brkitr/`): the compiled rule-based break
  iterator tables (`.brk`) that `ubrk_open(UBRK_LINE, locale)` loads, for
  `\XeTeXlinebreaklocale` (XeTeX_ext.c's `linebreakstart`,
  `crates/flashtex-xetex/src/native/linebreak.rs`, which runs them as ICU's
  `RuleBasedBreakIterator::handleNext` does). ICU's break-iterator code is
  not vendored.
- **The dictionary break engines' sources and dictionaries** (`icu-src/`,
  `brkitr/*dict.dict`): the source of record for
  `crates/flashtex-xetex/src/native/dictbe.rs`, which ports the Thai, Lao,
  Burmese and Khmer engines (issue #2432; see "The dictionaries and break
  engines" below).
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
| Path | `libs/icu/icu-src/source/data/in/icudt78l.dat` (sha256 `d7da8e2e31312a3cb574d1a0ebd7b187daa988e15a9b70b78eb68b213a590608`), items `brkitr/line*.brk` and `brkitr/{thai,lao,burmese,khmer}dict.dict`; `libs/icu/icu-src/source/common/` (the bidi files below); `libs/icu/version.ac` |
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

## The dictionaries and break engines

Each `brkitr/*dict.dict` is the item of that name in `icudt78l.dat`,
extracted by the data file's table of contents as `icupkg -x` does (the
same extraction reproduces `brkitr/line.brk` byte for byte). Each is a
`Dict` data file: a `BytesTrie` of ICU's word list with an offset
transform (the script's block start maps to byte 0).

| file | bytes | transform | engine |
|---|---|---|---|
| `thaidict.dict` | 126144 | `0x1000e00` | `ThaiBreakEngine` |
| `laodict.dict` | 162624 | `0x1000e80` | `LaoBreakEngine` |
| `burmesedict.dict` | 254448 | `0x1001000` | `BurmeseBreakEngine` |
| `khmerdict.dict` | 445552 | `0x1001780` | `KhmerBreakEngine` |

`icu-src/source/common/` also holds the files the engine port follows:

| file | contents |
|---|---|
| `dictbe.cpp`, `dictbe.h` | the four engines' `divideUpDictionaryRange`, `PossibleWord`, the engines' character sets |
| `dictionarydata.cpp`, `dictionarydata.h` | `BytesDictionaryMatcher::matches` and `transform`, the data file's indexes |
| `bytestrie.cpp`, `unicode/bytestrie.h` | the trie walk `first`/`next` that `matches` uses |

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

## The converters

A fourth part (issue #2382), also unmodified: `cnv/` holds ICU's converter
alias table `cnvalias.icu` and the 138 single-byte converter tables
(`.cnv` items whose MBCS header has one state, output type
`MBCS_OUTPUT_1` and no extension table) of the same `icudt78l.dat`
(sha256 `d7da8e2e31312a3cb574d1a0ebd7b187daa988e15a9b70b78eb68b213a590608`,
the pin above), each item extracted byte for byte from the package's table
of contents (as `icupkg -x` does). `\XeTeXinputencoding` and
`\XeTeXdefaultencoding` open a converter by name with `ucnv_open`, and
XeTeX_ext.c's `input_line` converts each line with `ucnv_toAlgorithmic`;
`crates/flashtex-xetex/src/ucnv.rs` ports the name lookup (`ucnv_bld.cpp`,
`ucnv_io.cpp` over `cnvalias.icu`), the single-byte conversion
(`ucnvmbcs.cpp`, with `swaplfnl`), and the algorithmic UTF-8, CESU-8,
ISO-8859-1, US-ASCII, UTF-16 and UTF-32 converters (`ucnv_u8.cpp`,
`ucnvlat1.cpp`, `ucnv_u16.cpp`, `ucnv_u32.cpp`), from ICU 78.2's sources
(TeX Live's `libs/icu/icu-src/source/common/`, not vendored).
`src/ucnv_sbcs.rs` lists the tables, and a test of the crate checks it
against `cnv/`.

Not vendored, and not ported (the port reports their names as unknown
encodings): the 52 other `.cnv` items (multi-byte tables such as
`ibm-943_P15A-2003` and `gb18030-2022`, extension-only ones, and
`gsm-03.38-2009` with its extension table), and ICU's algorithmic UTF-7,
IMAP-mailbox-name, SCSU, BOCU-1, ISO-2022, HZ, LMBCS, ISCII and
x11-compound-text converters.

## Licence

Unicode License v3 (`LICENSE`, ICU 78.2's), with the ICU licence for code
from before 2016. The port in `crates/flashtex-xetex` reproduces both
notices in that crate's `LICENSE`, as they require. It is a permissive
licence, and may be combined with the crate's GPL-2.0-or-later.
