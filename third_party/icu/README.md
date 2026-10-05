# `third_party/icu`

TeX Live 2026's own ICU, **as TeX Live has it**, for the Unicode mode's
bidirectional text, input encodings and line breaking
(`docs/design/xetex/PLAN.md` §3.1, phase S1). XeTeX calls ICU from
`XeTeX_ext.c` for:

| Use | XeTeX routine | ICU |
|---|---|---|
| Direction runs of a native word | `measure_native_node` | `ubidi_open`, `ubidi_setPara` (paragraph level `UBIDI_DEFAULT_LTR`/`_RTL` from `getDefaultDirection`), `ubidi_getDirection`, `ubidi_countRuns`, `ubidi_getVisualRun` |
| `\XeTeXinputencoding`, `\XeTeXdefaultencoding` other than `utf8`, `utf16`, `utf16be`, `utf16le`, `bytes`, `auto` | `getencodingmodeandinfo`, `setinputfileencoding`, `input_line` | `ucnv_open`, `ucnv_toAlgorithmic(UCNV_UTF32_NativeEndian, ...)`, `ucnv_close` |
| `\XeTeXlinebreaklocale` | `linebreakstart`, `linebreaknext` | `ubrk_open(UBRK_LINE, locale)`, `ubrk_setText`, `ubrk_next` |
| `--version` | `initversionstring` | `u_getVersion` |

`\XeTeXinputnormalization` is **not** ICU's: XeTeX's `apply_normalization`
normalises with TECkit (`TECkit_CreateConverter(NULL, 0, 1, UTF-32,
UTF-32 | kForm_NFC/kForm_NFD)`), so it is ported over `third_party/teckit`,
not here.

Built by `crates/flashtex-xetex/icu` (`flashtex-xetex-icu`).

## Pin

| | |
|---|---|
| Upstream | TeX Live source tree, GitHub mirror <https://github.com/TeX-Live/texlive-source> |
| Tag | `tags/texlive-2026.1` ("texlive-2026.1 tag based on r78399") |
| Commit | `6a300188053b8f2ded89dbd52293732a706b9c0e` (the same pin as `third_party/harfbuzz`, `third_party/freetype`, `third_party/xetex`) |
| Path | `libs/icu/icu-src/` |
| Version | 78.2 (`libs/icu/version.ac`; TeX Live imported `icu4c-78.2-sources.tgz` from <https://github.com/unicode-org/icu/releases/tag/release-78.2>, `libs/icu/TLpatches/TL-Changes`; `xetex --version` of TeX Live 2026: "Compiled with ICU version 78.2; using 78.2") |
| Fetched | 2026-10-04 |

`crates/flashtex-xetex/tests/pinned_libs.rs` fails if the linked library's
`u_getVersion` does not report the version above, if this table says
another, or if any file differs from `SHA256SUMS` or is missing from it.

TeX Live's changes to ICU 78.2 (`libs/icu/TLpatches/`): `patch-12-mingw` and
`patch-13-STATIC_PREFIX` edit build files (`source/config/mh-mingw`,
`source/config/Makefile.inc.in`, `source/icudefs.mk.in`), none of which is
vendored; `patch-18-uchar` edits `source/common/unicode/ptypes.h` (C only,
not C++: `<uchar.h>` is included only on Linux-based platforms). The vendored
`ptypes.h` is TeX Live's, with that patch. Every vendored file is
byte-identical to TeX Live's tree at the commit above (checked on
2026-10-04 with `shasum -a 256 -c` against a sparse checkout of
`libs/icu`).

## Files

**`icu-src/`** (426 files, 9.8 MB): `LICENSE` and `source/common/`, the
source of TeX Live's `libicuuc.a`: the 202 `.cpp` files of
`source/common/sources.txt`, every header of `source/common/` and
`source/common/unicode/` (222), and `sources.txt`. ICU's other libraries
(`i18n`, `io`, `tools`, the layout engine) are not used by XeTeX and not
vendored; nor are the build files (`*.vcxproj`, `BUILD.bazel`, `Makefile.in`,
`common.rc`) and the rule-builder's `rbbicst.pl`/`rbbirpt.txt` of
`source/common/`.

The Unicode data the bidi algorithm and the line breaker need for character
properties is compiled into the library (`ubidi_props_data.h`,
`uchar_props_data.h`, `ucase_props_data.h`, `propname_data.h`, ...), so it is
part of `icu-src/` and needs no data file.

**`icudt78l/`** (37 files, 3.8 MB): items of TeX Live's ICU data package,
`libs/icu/icu-src/source/data/in/icudt78l.dat` (21,953,264 bytes, sha256
`d7da8e2e31312a3cb574d1a0ebd7b187daa988e15a9b70b78eb68b213a590608`), which
TeX Live links into `xetex` whole (`libicudata.a`). The items, unchanged:

* `cnvalias.icu`, the converter alias table. Without it ICU opens a
  converter only by its canonical name with punctuation removed
  (`iso88591`, `usascii`, `utf8`): `latin1`, `ascii`, `ISO-8859-1`,
  `UTF-8` and every other alias would not open.
* `brkitr/*`, the whole break-iteration tree: the line-break rules
  (`line*.brk`), the dictionaries of the scripts written without spaces
  (`thaidict`, `laodict`, `khmerdict`, `burmesedict`, `cjdict`), and the
  locale bundles that choose among them (`root.res`, `ja.res`, ...). The
  word, sentence, character and title rules (`word*.brk`, `sent*.brk`,
  `char.brk`, `title.brk`, 112 KB) are never used by XeTeX and are kept so
  that the tree is the whole of TeX Live's `brkitr`.

Each item is the bytes between its offset and the next item's in the
package's table of contents (items are padded to 16 bytes), which is what
`icupkg -x` writes. `extract-data.py` here does it:

```sh
python3 third_party/icu/extract-data.py <texlive-source>/libs/icu/icu-src/source/data/in/icudt78l.dat
```

| Item | Bytes | sha256 |
|---|---|---|
| `brkitr/burmesedict.dict` | 254448 | `99f1b2afe6672bc495a872e85dd5ed2be5ffc4d9b8377deff2dbc18a2b4538e5` |
| `brkitr/char.brk` | 14048 | `5daa5d70b011e407f0c4ba06498792b224852f911991e14f0d88a4bef19feaa1` |
| `brkitr/cjdict.dict` | 2007296 | `3850478c713b2004cbb96ef4b9f049caf16d6faf32a883d2e63c09659d0c59c7` |
| `brkitr/de.res` | 2992 | `a08312a9303b1373f7129bdb6ebd798cb39073476f75d30f1602c0c6a99c05b2` |
| `brkitr/el.res` | 160 | `de0af623cf2c0440bfd567bb89116f02c78dc666b7fe3f62baaa47315937ebe3` |
| `brkitr/en.res` | 1776 | `74c5fbfbfc3517d5383a84f9cbfbd0e3ab7c47fbdd09320d0429938fdb6ca984` |
| `brkitr/en_US.res` | 80 | `ee066818c0bfc2e92c9f4486f3087c3669b998786b0363a04467136d80c00d8b` |
| `brkitr/en_US_POSIX.res` | 160 | `f0b36c871bf8cde086d24478f43c465de47a62ad267922022f0b2cf29bc5f94c` |
| `brkitr/es.res` | 2176 | `4db4b3ef626368fa010b546855296c5fa1556e5843d62fa5647e19bbe25cfdc6` |
| `brkitr/fr.res` | 1200 | `d95b1d5e778b175096289618ba726c7f31844609cdd1a69b5bb6748873f2d7ce` |
| `brkitr/it.res` | 688 | `86b433f7a2456522c6c9310e6a23d7783e414f72c66dd5d4643096225caeec6a` |
| `brkitr/ja.res` | 2784 | `2cf394b51397cf3bc8f172275e1a956ee243658caa1120ef13c809ba276aeb5a` |
| `brkitr/khmerdict.dict` | 445552 | `080582d61636c7f9039d65bc37452d100faa07f3ba7f5e21aa32aa1672441413` |
| `brkitr/ko.res` | 544 | `7af4ffd62b4dffaf18f453701fd37361f6c3c2d05a1edce201c45f37949f882c` |
| `brkitr/laodict.dict` | 162624 | `b044d4232c2fea926b88f6047c4efcd683e7cf0573cf9f60990aa7500c60ef0c` |
| `brkitr/line.brk` | 73424 | `69e557c954e18af5be4365ce39050d42eabfbf63d3b8986659e6e1bc16271805` |
| `brkitr/line_cj.brk` | 73456 | `801fd1a48cf89d3a88a9a9725e6f6b73ba58ce50dc0f3cc8131a1046bedef4dc` |
| `brkitr/line_loose.brk` | 75584 | `2e785ac63d53b5ec97b279320d0419fd2404747fb54ca6e5f8f805ba606ea6c3` |
| `brkitr/line_loose_cj.brk` | 80240 | `c92fa675b1288d693dcfaaff419919a1cb4330066f4fc9140346790222d0a9f6` |
| `brkitr/line_loose_phrase_cj.brk` | 85584 | `367a321f42d11fe2b711570e6a379e1b81bf1ed7eecf59342096c89d974d6c71` |
| `brkitr/line_normal.brk` | 73312 | `ae0c4188acc2e4be6d282718ce081a2dd8c4b1e827a54fbb800756db6617a6d5` |
| `brkitr/line_normal_cj.brk` | 74416 | `6202891a973d0e757c8d6e039b1f19a6e630d123b358802bda1f47c763475217` |
| `brkitr/line_normal_phrase_cj.brk` | 79552 | `695cd7a4dbecb9aea13c86333c607d35534c64f93e73ce5aa9f6584a8096539b` |
| `brkitr/line_phrase_cj.brk` | 78560 | `0a62eeddacafcc1ff06be9524bf38086414774c5042b3bc6da0d524e4109d047` |
| `brkitr/pt.res` | 2288 | `200c5c8d1a5d1ef45a57b40517cb966842ce9fa63d0147f8f6fb210a8a8dbefe` |
| `brkitr/res_index.res` | 240 | `440e175bbb60400c8739924222d63d926859e9fa72d2c38bbd88b8b30313c339` |
| `brkitr/root.res` | 800 | `863bd431dd5e16f82af96366116f1e1b1aa78a04536ebb74ef97e60a45e8bbd5` |
| `brkitr/ru.res` | 336 | `9e934ecf7041a7aead1c5aa813582e36b21565fb626e762cf6247c709d936614` |
| `brkitr/sent.brk` | 19856 | `11d022b98e12bb0bb0533657d1888172cf1e15ae1539b2edb42ad5158e40521b` |
| `brkitr/sent_el.brk` | 19872 | `cb7c92138052336827287897334404debe7ed35b5cb7a971d362149b6ac6e17a` |
| `brkitr/thaidict.dict` | 126144 | `ea47ae521cea65077bf047a75d1e9d28c495ec2f8cf4d2734fcb65a888229be9` |
| `brkitr/title.brk` | 12400 | `413bc316a791fc38812e961c356cbaea8bce9975190a16357f72c69de09c0f01` |
| `brkitr/word.brk` | 23120 | `659f2a18f7ecec07b498d392bb3e7ca03c8466fffa19a83d1db215007a826898` |
| `brkitr/word_POSIX.brk` | 23120 | `01530a62f2701ce83d5465eaed38e79ae4afa867a59408be7c8dc1cf79b04ade` |
| `brkitr/zh.res` | 272 | `35e7c34f5b94dda401bfc8d79c88dacc80a6985d821df44b5b8b667eacfe3646` |
| `brkitr/zh_Hant.res` | 272 | `35e7c34f5b94dda401bfc8d79c88dacc80a6985d821df44b5b8b667eacfe3646` |
| `cnvalias.icu` | 64016 | `e833f51cb0a0874388cfef50c165af672cf2178c497b9d1bc70270744becf24b` |

Checksums of every vendored file, sources and data, are in `SHA256SUMS`:

```sh
cd third_party/icu && shasum -a 256 -c SHA256SUMS
```

### What is left out of the data, and what it changes

The other 2,007 items of the package are not linked: 5,150,336 bytes of
table-driven converters (`*.cnv`), and locale, collation, unit,
transliteration, number-spellout and normalisation (`nfkc*.nrm`, used by
XeTeX not at all) data that XeTeX never reaches. Of these only the `.cnv`
tables change what XeTeX does: with them TeX Live's `xetex` reads
`\XeTeXinputencoding "latin2"`, `"cp1252"`, `"macintosh"`, `"koi8-r"`,
`"shift_jis"`, `"gb18030"` and the other table-driven encodings; FlashTeX's
ICU resolves the alias (`cnvalias.icu`) but cannot load the table
(`U_FILE_ACCESS_ERROR`), and XeTeX's own handling of a converter ICU cannot
open follows: the diagnostic ``Unknown encoding `latin2'; reading as raw
bytes`` and the file read as bytes. Measured with TeX Live 2026's `xetex`
(the lockstep case `b014-encoding-tables`): byte 0xB1 under `latin2` is
U+0105 (261) there and 177 here, 0x80 under `cp1252` U+20AC (8364) against
128, 0x8E under `macintosh` U+00E9 (233) against 142. The algorithmic
converters, which need no table (UTF-8, UTF-16, UTF-32 and their endian
forms, UTF-7, ISO-8859-1, US-ASCII, SCSU, BOCU-1, CESU-8, IMAP mailbox
names), behave as in TeX Live under every alias.

Linking TeX Live's whole package instead would add its 21,953,264 bytes to
the binary (about 18 MB more than this subset); adding only the `.cnv`
tables, 5,150,336 bytes.

## Build

`crates/flashtex-xetex/icu/build.rs` compiles the 202 sources of
`source/common/sources.txt` as C++ with the flags of TeX Live's build, which
runs ICU's own `source/configure` with `--enable-static --disable-shared
--disable-extras --disable-samples --disable-tests --disable-dyload
--disable-layout` (`libs/icu/configure.ac`). That configure, run unmodified
on macOS on 2026-10-04, writes `icudefs.mk` with:

| Flag | Source | Effect |
|---|---|---|
| `-DU_ALL_IMPLEMENTATION -DU_ATTRIBUTE_DEPRECATED=` | `DEFS`, `icudefs.mk` | ICU's own build of its libraries |
| `-DU_ENABLE_DYLOAD=0` | `CPPFLAGS` (`--disable-dyload`) | no plugin loading |
| `-DU_COMMON_IMPLEMENTATION -I source/common` | `source/common/Makefile` | building `libicuuc` |
| `-DDEFAULT_ICU_PLUGINS="/usr/local/lib/icu"` | `source/common/Makefile` (`$(libdir)/icu`, the default prefix) | unused: plugins are disabled |
| `-O2 -std=c++17` | `CXXFLAGS` | applied whatever the cargo profile |
| `-fvisibility=hidden -fno-common` | `LIBCXXFLAGS`, `COMPILE.cc` of `config/mh-darwin` | symbol visibility only |

It writes no header: ICU 78's `platform.h` and `uconfig.h` are not
generated, so the library is configured by their defaults, including
ICU's renaming of every function with the major version (`urename.h`:
`ubidi_open` is the symbol `ubidi_open_78`), which the Rust FFI links. The
data is TeX Live's default `static` packaging: `pkgdata` turns the package
into an object whose symbol is `icudt78_dat` (`U_ICUDATA_ENTRY_POINT`) with
`genccode`, and so does `build.rs`: it packs the items above into a package
of the same format (a `CmnD` 1.0 header, the table of contents sorted by
name, each item at a 16-byte boundary; TeX Live's package is sorted the same
way) and writes `genccode`'s C for it (`tools/toolutil/pkg_genc.cpp`,
`writeCCode`). One entry is added: a sentinel with no data after the last
item, because ICU knows an item's length only from where the next one
starts, and `ucnv_io` rejects an alias table of unknown length (in TeX
Live's package the last item is `zu_ZA.res`, which XeTeX never reads). With
no `ICU_DATA` set, ICU finds no data files on disk and uses the linked
package, as TeX Live's `xetex` does.

Measured on 2026-10-04 (M1 Max, load average about 160, `CARGO_BUILD_JOBS=4`):
a clean `cargo build --release -p flashtex-xetex-icu` takes 2 min 3 s (74 s of
CPU); the release `flashtex-xetex` binary grows from 3,483,216 to 8,641,184
bytes (+5,157,968: the 3.9 MB data package and about 1.3 MB of ICU code).

## Licence

ICU is under the **Unicode License v3** (`icu-src/LICENSE`, SPDX
`Unicode-3.0`), a permissive, MIT-like licence that is GPL-compatible.
Checked on 2026-10-04, file by file:

- 424 of the 425 files of `icu-src/source/common/` carry ICU's notice
  ("License & terms of use: http://www.unicode.org/copyright.html");
  `sources.txt`, a file list, carries none. `LICENSE` covers all of them.
- The data items carry no notice of their own. `LICENSE`'s "Third-Party
  Software Licenses" section covers the dictionaries that have other
  terms: `cjdict` (BSD licence, Google; its word lists from Libtabe, under
  a BSD-style licence, and IPADIC, under a permissive licence that asks for
  its notice), `laodict` (ICU's licence and a BSD-style one) and
  `burmesedict` (BSD-style). The Thai and Khmer dictionaries are ICU's
  own. `LICENSE` is vendored and ships with the source.
- **No Apache-2.0 code**: a search of every vendored file for `Apache`,
  `GPL-3`, `LGPL`, `Mozilla Public`, `MPL` and `GNU General` matches only
  `LICENSE`, whose GPL text belongs to build files that are not vendored
  (`aclocal.m4`'s `pkg.m4`, GPL-2.0-or-later with the Autoconf exception,
  and `config.guess`, GPL-3.0-or-later with the Autoconf exception). The
  other third-party parts `LICENSE` lists are outside `source/common/` and
  not vendored: double-conversion (BSD-3-Clause, `i18n`), nlohmann/json (MIT,
  `tools`), the time-zone database (public domain, `data`), `install-sh`
  and `sorttable.js`.
