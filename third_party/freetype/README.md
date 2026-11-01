# `third_party/freetype`

TeX Live 2026's own FreeType, **unmodified**, for the Unicode mode's font
loading and glyph metrics (`docs/design/xetex/PLAN.md` §3.1, phase S1). XeTeX
opens every native font with FreeType and takes advances, bounding boxes,
kerning, glyph names and the `OS/2`, `post` and `head` tables from it
(`XeTeXFontInst.cpp`), so its output depends on FreeType's exact version;
FlashTeX uses the one TeX Live 2026's `xetex` is built with. Built by
`crates/flashtex-xetex/fontlibs` (`flashtex-xetex-fontlibs`).

## Pin

| | |
|---|---|
| Upstream | TeX Live source tree, GitHub mirror <https://github.com/TeX-Live/texlive-source> |
| Tag | `tags/texlive-2026.1` ("texlive-2026.1 tag based on r78399") |
| Commit | `6a300188053b8f2ded89dbd52293732a706b9c0e` (the same pin as `third_party/pdftex`, `third_party/zlib`, `third_party/xetex`) |
| Path | `libs/freetype2/freetype-src/` |
| Version | 2.14.1 (`libs/freetype2/version.ac`; TeX Live imported it from <http://download.savannah.gnu.org/releases/freetype> on 2025-09-12, `libs/freetype2/TLpatches/ChangeLog`) |
| Fetched | 2026-10-04 |

`crates/flashtex-xetex/tests/pinned_libs.rs` fails if `FT_Library_Version`
does not report the version above, if this table says another, or if any
file differs from `SHA256SUMS`. TeX Live applies no patch to this tree
(`libs/freetype2/TLpatches/TL-Changes` lists none).

## Files

`freetype-src/` holds 435 files (7.2 MB): every file TeX Live's
`libfreetype.a` compiles (the 43 of `freetype::SOURCES` in
`crates/flashtex-xetex/fontlibs/build.rs`, read from `make -n` of TeX Live's
configured build) with every file they include under that configuration (the
compiler's dependency list, `-MM`), all public headers of
`include/freetype/` and the `include/freetype/config/` headers except the
generic `ftconfig.h` and `ftmodule.h` (the build generates both, below),
`builds/unix/ftconfig.h.in`, `src/base/ftsystem.c` (used on Windows, where
`builds/unix/ftsystem.c` does not apply), the generator inputs of two
generated sources (`src/autofit/afblue.{dat,cin,hin}` with
`src/tools/afblue.pl`, and `src/tools/glnames.py` for `src/psnames/pstables.h`;
they are the preferred form of those files for modification, which the GPL
option asks for), and the licence files: `LICENSE.TXT`, `docs/FTL.TXT`,
`docs/GPLv2.TXT`, `src/bdf/README`, `src/pcf/README`, plus `README`.
Checksums of every file are in `SHA256SUMS`:

```sh
cd third_party/freetype && shasum -a 256 -c SHA256SUMS
```

Left out of TeX Live's 855-file tree: the other build systems (`builds/` but
two files, CMake, meson, MSBuild, VMS, `configure`, `modules.cfg` and the
`rules.mk`/`module.mk` files), `docs/` but the licences, `devel/`, `tests/`,
`subprojects/`, `src/tools/` but the two generators, the module sources that
the configuration does not include (for example `src/gzip/gzguts.h`,
`src/dlg/dlg.c` and four headers of the HarfBuzz-backed autofit code, which
compile only with `FT_DEBUG_LOGGING` or `FT_CONFIG_OPTION_USE_HARFBUZZ`), and the module
`README`s other than BDF's and PCF's.

## Build

TeX Live builds FreeType with FreeType's own `builds/unix` `configure`
(`libs/freetype2/Makefile.am`, target `ft-config`):

```
configure --disable-shared --without-bzip2 --without-brotli \
          --without-harfbuzz --without-png --without-zlib
```

`crates/flashtex-xetex/fontlibs/build.rs` reproduces what that configure
produces (checked by running it, unmodified, on macOS on 2026-10-04) and
compiles the same 43 files, in `make`'s order, with the `cc` crate:

| What | TeX Live's configured build | here |
|---|---|---|
| `ftoption.h` | configure's copy of `include/freetype/config/ftoption.h`, unchanged for these options (it edits only the bzip2/png/harfbuzz/brotli/zlib lines when one is enabled) | the vendored file, in place (`-DFT_CONFIG_OPTIONS_H=<freetype/config/ftoption.h>`) |
| `ftconfig.h` | `builds/unix/ftconfig.h.in` with `#define HAVE_UNISTD_H 1` and `#define HAVE_FCNTL_H 1` | generated the same way into `$OUT_DIR` on Unix hosts; on Windows the two stay `#undef` |
| `ftmodule.h` | generated from the default `modules.cfg`: drivers `truetype type1 cff cid pfr type42 winfonts pcf bdf`, `sfnt`, hinters `autofit pshinter`, renderers `smooth raster svg sdf` (+ `bitmap_sdf`), `psaux`, `psnames`, in that order | written byte for byte by `build.rs` |
| Defines | `-DFT2_BUILD_LIBRARY -DFT_CONFIG_CONFIG_H="<ftconfig.h>" -DFT_CONFIG_MODULES_H="<ftmodule.h>" -DFT_CONFIG_OPTIONS_H="<ftoption.h>"`, and `-DDARWIN_NO_CARBON` on macOS | the same |
| Flags | `-pedantic -std=c99 -O2 -fvisibility=hidden -pthread` (`-g` and `-Wall` aside) | `-std=c99 -O2 -fvisibility=hidden`, whatever the cargo profile; `-pedantic` only changes diagnostics, and no vendored FreeType file uses threads (`-pthread` is configure's default for `dlg` logging) |
| `ftsystem.c` | `builds/unix/ftsystem.c` (memory-mapped files) | the same; `src/base/ftsystem.c` on Windows |

The resulting options (from the unchanged `ftoption.h`): the internal copy of
zlib in `src/gzip` (`FT_CONFIG_OPTION_USE_ZLIB` without
`FT_CONFIG_OPTION_SYSTEM_ZLIB`: what `--without-zlib` gives), LZW, no bzip2,
PNG, Brotli or HarfBuzz; PostScript names and the Adobe Glyph List; Mac fonts
and resource forks; incremental loading; SVG and COLR; the TrueType bytecode
interpreter with minimal subpixel hinting; TrueType GX/OpenType variations
(`TT_CONFIG_OPTION_GX_VAR_SUPPORT`); the CJK and Indic autofit scripts; and
`FT_CONFIG_OPTION_ENVIRONMENT_PROPERTIES`, under which `FT_Init_FreeType`
reads the `FREETYPE_PROPERTIES` environment variable, exactly as TeX Live's
`xetex` does (it changes hinting and rendering properties, not the unscaled
metrics XeTeX asks for).

## Licence

FreeType is dual-licensed (`freetype-src/LICENSE.TXT`): the FreeType License
(`docs/FTL.TXT`) **or** "the GNU General Public License version 2, found in
`docs/GPLv2.TXT` (any later version can be used also)". **FlashTeX uses
FreeType under the GPL-2.0-or-later option** (owner ruling, 2026-10-04): the
FTL's advertising clause is incompatible with GPLv2, and the XeTeX port and
the binary that links it are GPL. `crates/flashtex-xetex/fontlibs` declares
`GPL-2.0-or-later AND MIT` accordingly (MIT for HarfBuzz).

Checked on 2026-10-04, file by file (a scan of every vendored file for its
notice, plus a search for `Apache`, `GPL-3`, `LGPL`, `MPL` and
`SPDX-License-Identifier`):

- 399 files carry the FreeType project licence notice; 21 of them (the Adobe
  CFF engine in `src/psaux/` and its headers) add Adobe's patent grant under
  that licence;
- 18 files, the BDF and PCF drivers (`src/bdf/`, `src/pcf/`) and `src/base/fthash.c`,
  `include/freetype/internal/fthash.h` are under X11-style permissive
  licences (MIT/Expat and The Open Group's; with `src/bdf/README` and
  `src/pcf/README`), which `LICENSE.TXT` says are
  compatible with both options;
- 12 files of `src/gzip/` are zlib under the zlib licence (`src/gzip/zlib.h`);
  `crc32.h` and `inffixed.h` there are zlib's generated tables, with no notice
  of their own;
- `src/autofit/ft-hb-ft.c` is HarfBuzz code under the Old MIT licence (its
  notice is in the file; it compiles only with `FT_CONFIG_OPTION_USE_HARFBUZZ`,
  off here);
- `include/freetype/ftchapters.h` is a documentation-only header with no
  notice (covered by `LICENSE.TXT`); the other files without a notice are
  the licence texts themselves (`LICENSE.TXT`, `docs/GPLv2.TXT`);
- **No Apache-2.0 code, and no GPL-3-only, LGPL or MPL code**: the only
  matches are `LICENSE.TXT`'s own remark that the FTL is "compatible to the
  GNU General Public License version 3, but not version 2", and font version
  strings ("version 3.00") in `src/truetype/ttobjs.c` and `src/sfnt/ttsbit.c`.
