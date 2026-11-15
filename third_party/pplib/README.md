# `third_party/pplib`

TeX Live 2026's own pplib, **unmodified**: Paweł Jackowski's PDF reader,
written for LuaTeX, which XeTeX has used instead of poppler since TeX Live
2021 (`texk/web2c/xetexdir/pdfimage.cpp`) to read a PDF picture's page count
and page boxes for `\XeTeXpdffile` and `\XeTeXpdfpagecount`. Built by
`crates/flashtex-xetex/pplib` (`flashtex-xetex-pplib`).

Why the library itself and not a reading of the same boxes through xpdf
(which the port used until flash-tex/flashtex#2494): whether a PDF file can
be read **at all** is the reader's own behaviour. `ppdoc_load` returns NULL
for a file whose `startxref` does not point at its cross-reference table,
that has no `startxref`, that has bytes before `%PDF-`, or bytes after
`%%EOF`; xpdf repairs all four. For such a file XeTeX reports
`\XeTeXpdfpagecount` 0 and `! Unable to load picture or PDF file`, so the
port must too (16 figures of arXiv 2501.07069v1 are such files).

## Pin

| | |
|---|---|
| Upstream | TeX Live source tree, GitHub mirror <https://github.com/TeX-Live/texlive-source> |
| Tag | `tags/texlive-2026.1` ("texlive-2026.1 tag based on r78399") |
| Commit | `6a300188053b8f2ded89dbd52293732a706b9c0e` (the same pin as `third_party/pdftex`, `third_party/zlib`, `third_party/xetex`, `third_party/teckit`) |
| Path | `libs/pplib/pplib-src/` (and `libs/pplib/Makefile.am`, `version.ac`, `ChangeLog` under `tl/`) |
| Version | 2.05.0 (`libs/pplib/version.ac`); the library's own `pplib_version` is `"v2.2"` (`src/ppapi.h`) |
| Fetched | 2026-11-15 |

`crates/flashtex-xetex/tests/pinned_libs.rs` fails if this table pins
another version, if any file differs from `SHA256SUMS` or is missing from
it, or if the linked library's `pplib_version` is not `v2.2`.

TeX Live applies no patches to pplib (`libs/pplib` has no `TLpatches/` at
this commit, though its Makefile.am names one).

## Files

`pplib-src/src/` is TeX Live's directory whole: the 21 C files of TeX
Live's `libpplib.a` (`nodist_libpplib_a_SOURCES` in `tl/Makefile.am`), their
headers, and the three test programs `pptest1.c`, `pptest2.c`, `pptest3.c`,
which TeX Live also lists and which are not compiled here (each is a `main`;
no program refers to them). `pplib-src/README.md` is upstream's. `tl/` holds
TeX Live's `Makefile.am`, `version.ac` and `ChangeLog` for reference. 62
files, 780 KB. Checksums of every file are in `SHA256SUMS`:

```sh
cd third_party/pplib && shasum -a 256 -c SHA256SUMS
```

Left out: `pplib-src/doc/` (the manual and its built HTML),
`pplib-src/test/` (a sample PDF and the test programs' expected output),
`CMakeLists.txt`, `azure-pipelines.yml` and TeX Live's autotools files.

## Build

`crates/flashtex-xetex/pplib/build.rs` compiles the 21 C files with the `cc`
crate, with TeX Live's flags:

| Flag | Source | Effect |
|---|---|---|
| `-I src -I src/util` | `AM_CPPFLAGS` | pplib's headers |
| zlib's headers, `-DZ_PREFIX` | `ZLIB_INCLUDES` | `zlib.h`: TeX Live's zlib 1.3.2 (`third_party/zlib`), as the TECkit crate builds it; the objects are that crate's |
| `-O2` | autoconf's default `CFLAGS` (`-g -O2`) | applied whatever the cargo profile |
| (none) | `PPLIB_DEFINES` (`-DHAVE_UNISTD_H` and the large-file macros) | no pplib source reads them |
| (warnings) | `WARNING_CFLAGS` | warnings only; silenced here |

No `-DNDEBUG`, as TeX Live gives none: the only `assert`s are in the test
programs. `-lm` as TeX Live's `LIBS`.

With it, `crates/flashtex-xetex/pplib/csrc/flashtex_pdfimage.c`: XeTeX's
`pdfimage.cpp` (`pdf_get_rect`, `pdf_count_pages`) as C, statement for
statement, except that a page pplib cannot find, or whose object is not a
dictionary, gives -1 where XeTeX dereferences it and crashes (its header
says which inputs).

pplib keeps process-wide state (filter heaps, the logger's buffer), so the
crate makes every call under one lock.

## Licence

pplib's tree has **no licence text**: neither TeX Live's `libs/pplib` nor
upstream (<https://github.com/contextgarden/pplib>) carries a `LICENSE` or
`COPYING` file, and the sources have no licence headers
(only `src/util/utilmd5.c`, Aladdin Enterprises' MD5 under its zlib-style
licence, and `src/util/utilsha.c`, Aaron D. Gifford's SHA-2 under a BSD
licence, carry notices of their own). pplib was written for LuaTeX and distributed inside its
source tree (`luatexdir/luapplib`, moved to `libs/pplib` in 2020, `tl/ChangeLog`);
LuaTeX is GPL-2.0-or-later. Debian's `texlive-bin` copyright file records it
so: `Files: libs/pplib/*`, `Copyright (C) 2019-2023 Paweł Jackowski`,
`License: GPL-2+`.

FlashTeX uses pplib under **GPL-2.0-or-later** and links it only into the
XeTeX port, which is GPL-2.0-or-later already (`crates/flashtex-xetex`).
Nothing of it is linked into the MIT crates or the app.

This reading rests on LuaTeX's licence and Debian's record, not on a
statement in the tree: it is flagged for the owner's confirmation in the
pull request that vendored it.
