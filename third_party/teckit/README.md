# `third_party/teckit`

TeX Live 2026's own TECkit conversion engine, **unmodified**, for the Unicode
mode's font mappings (`docs/design/xetex/PLAN.md` §3.1, phase S1). XeTeX
applies a font's `mapping=` (a compiled `.tec` file such as `tex-text.tec`:
` `` ` to “, `--` to –, ...) and `\XeTeXinputnormalization` with this
engine (`XeTeX_ext.c`'s `load_mapping_file`, `applymapping`,
`applytfmfontmapping`, `apply_normalization`), so FlashTeX uses the TECkit
TeX Live 2026's `xetex` is built with. Built by
`crates/flashtex-xetex/teckit` (`flashtex-xetex-teckit`).

## Pin

| | |
|---|---|
| Upstream | TeX Live source tree, GitHub mirror <https://github.com/TeX-Live/texlive-source> |
| Tag | `tags/texlive-2026.1` ("texlive-2026.1 tag based on r78399") |
| Commit | `6a300188053b8f2ded89dbd52293732a706b9c0e` (the same pin as `third_party/pdftex`, `third_party/zlib`, `third_party/xetex`, `third_party/harfbuzz`) |
| Path | `libs/teckit/TECkit-src/` and `libs/teckit/TLpatches/` |
| Version | 2.5.13 (`libs/teckit/version.ac`; TeX Live imported <https://github.com/silnrsi/teckit/archive/refs/tags/v2.5.13.tar.gz> on 2025-12-31, `libs/teckit/TLpatches/ChangeLog`) |
| Fetched | 2026-10-04 |

`crates/flashtex-xetex/tests/pinned_libs.rs` fails if this table pins
another version, if any file differs from `SHA256SUMS` or is missing from
it, or if the linked engine's `TECkit_GetVersion()` is not 0x00020004 (the
engine API version, 2.4, which every release since 2006 reports, 2.5.13
included; the release itself is pinned by this table and the checksums).

`TECkit-src/` is TeX Live's tree, in which TeX Live's patches are already
applied: `TLpatches/` (vendored for reference) records them.
`patch-07-warnings-and-static` removes `DllMain` from `source/Engine.cpp`
and, on Windows, redefines `WINAPI`/`EXPORTED` in
`source/Public-headers/TECkit_Engine.h` for a static library; its other
hunks and `patch-06`/`patch-09` touch the compiler and the tools, which are
not vendored.

## Files

`TECkit-src/` holds what TeX Live's `libTECkit.a` compiles
(`nodist_libTECkit_a_SOURCES` in `libs/teckit/Makefile.am`: only
`source/Engine.cpp`, which `#include`s `source/NormalizationData.c`) and
every file it includes (the compiler's own dependency list, `-MM`):
`source/Engine.h`, `source/TECkit_Format.h`,
`source/Public-headers/TECkit_Engine.h`, `TECkit_Common.h`; plus the
licence files (`COPYING`, `license/`), `AUTHORS`, `README`, `README.md`,
`NEWS` and `ChangeLog`. 20 files with `TLpatches/`, 516 KB. Checksums of
every file are in `SHA256SUMS`:

```sh
cd third_party/teckit && shasum -a 256 -c SHA256SUMS
```

Left out: the mapping compiler (`source/Compiler.cpp`,
`UnicodeNames.cpp`: TeX Live's `libTECkit_Compiler.a` and `teckit_compile`,
which XeTeX does not link), `sfconv`/`txtconv` and the other sample tools,
the Perl, REALbasic and JNI interfaces, the tests, the docs and the
autotools files. `tex-text.map`/`tex-text.tec` (TeX Live's
`libs/teckit/`) are not vendored: the `.tec` files are TeX Live content,
found through kpathsea's `misc fonts` path (TeX Live's or the no-TeX-Live
bundle's, PLAN.md §3.4).

## Build

`crates/flashtex-xetex/teckit/build.rs` compiles `source/Engine.cpp` with the
`cc` crate as C++, with TeX Live's flags:

| Flag | Source | Effect |
|---|---|---|
| `-DHAVE_CONFIG_H` | automake `DEFS` | `Engine.cpp` includes `config.h` |
| `-DNDEBUG` | `AM_CPPFLAGS`, `libs/teckit/Makefile.am` | no assertions |
| `-I source/Public-headers` | `AM_CPPFLAGS` | the public headers |
| `-I <config dir>` | `DEFAULT_INCLUDES` | `config.h` |
| zlib's headers | `ZLIB_INCLUDES` | `zlib.h`: TeX Live's zlib (below) |
| `-O2` | autoconf's default `CXXFLAGS` (`-g -O2`) | applied whatever the cargo profile |
| (warnings) | `WARNING_CXXFLAGS` (`-Wreturn-type -Wno-write-strings`) | warnings only; silenced here |

C++ exceptions stay on, as in TeX Live: the engine reports a bad mapping by
throwing inside `TECkit_CreateConverter`, which catches it and returns a
status. No `-std` is given, as TeX Live gives none.

`config.h` is `crates/flashtex-xetex/teckit/config/config.h`: the file TeX
Live's `libs/teckit/configure` writes (run unmodified on macOS on
2026-10-04), except that `WORDS_BIGENDIAN` is decided per target from the
compiler's `__BYTE_ORDER__`. It is the only define `Engine.cpp` reads (the
compiled tables are big-endian on disk); every FlashTeX target is
little-endian, as configure found on macOS. TeX Live's configure also
defines `HAVE_LIBZ`, `HAVE_ZLIB_H` and `ZLIB_CONST`, which the file keeps.

**zlib.** `Engine.cpp` calls zlib's `uncompress` for compressed mappings
(all 85 `.tec` files under TeX Live 2026's `fonts/misc/xetex/fontmapping`
are compressed, magic `zQmp`). TeX Live
links its own zlib (`KPSE_ZLIB_FLAGS`, `libs/zlib`); so does this build: the
same `third_party/zlib` tree (zlib 1.3.2) as `crates/flashtex-engine`,
compiled the same way (the `libz.a` sources except `gz*.c`, `zconf.h`
copied from `zconf.h.in`, `Z_PREFIX`, which renames the symbols to `z_*` and
changes no code). The XeTeX port links the engine crate too, whose archive
has the same objects; the linker takes each object from one archive, never
both (see the build script).

## Licence

TECkit is "Copyright 2002-2025, SIL Global", distributable "under the terms
of either a) the Common Public License ... version 0.5 ... or (at your
option) any later version, or b) the GNU Lesser General Public License ...
version 2.1 of License, or (at your option) any later version"
(`TECkit-src/license/LICENSING.txt`; every vendored source file's header
says the same). **FlashTeX uses it under the LGPL-2.1-or-later**, which is
GPL-compatible (the CPL is not), and links it only into the GPL XeTeX port
(`crates/flashtex-xetex`, GPL-2.0-or-later), as PLAN.md §3.1 says.
`source/NormalizationData.c` has no header of its own: it is generated by
TECkit's `MakeNormData.pl` from the Unicode Character Database (17.0.0,
`NEWS`), and falls under the project licence above (`LICENSING.txt`: "Rest
of the project"). The licence exceptions `LICENSING.txt` names
(`SFconv/UtfCodec.*`, `docs/*.1`) are not vendored. No Apache-2.0-only code is
vendored.
