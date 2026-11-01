# `third_party/libpng`

TeX Live 2026's own libpng, **unmodified**, for the engine's PNG inclusion
(`crates/flashtex-engine/src/pdftex/writepng.rs`, the port of pdfTeX's
`writepng.c`). pdfTeX decodes PNG rows, strips 16-bit samples and alpha,
expands transparency and applies gamma with libpng, so the same library at
the same version gives the same image bytes; see
`docs/evidence/pdf-images-2026-09-29/README.md` for why it is linked rather
than ported.

## Pin

| | |
|---|---|
| Upstream | TeX Live source tree, GitHub mirror <https://github.com/TeX-Live/texlive-source> |
| Tag | `tags/texlive-2026.1` ("texlive-2026.1 tag based on r78399") |
| Commit | `6a300188053b8f2ded89dbd52293732a706b9c0e` (the same pin as `third_party/pdftex` and `third_party/zlib`) |
| Path | `libs/libpng/libpng-src/` |
| Version | libpng 1.6.55 (`libs/libpng/version.ac`); TeX Live 2026's `pdftex --version` says "Compiled with libpng 1.6.55; using libpng 1.6.55" |
| Fetched | 2026-09-29 |

TeX Live's own changes to the libpng 1.6.55 release (`libs/libpng/TLpatches/TL-Changes`)
are that `pnglibconf.h` is a copy of `scripts/pnglibconf.h.prebuilt` and that
build-system files are removed; the vendored files are TeX Live's.

## Files

`libpng-src/` holds the files of TeX Live's `libpng.a`
(`nodist_libpng_a_SOURCES` in `libs/libpng/Makefile.am`: the fifteen `png*.c`
and, on ARM hosts, the `arm/` NEON files), the headers they include, and
`LICENSE`, `README` and `AUTHORS`. Nothing else of the tree (tests,
contrib, other CPU ports) is vendored. Checksums of every file are in
`SHA256SUMS`, and each file is byte-identical to TeX Live's:

```sh
cd third_party/libpng && shasum -a 256 -c SHA256SUMS
```

## Build

`crates/flashtex-engine/build.rs` compiles the sources with the `cc` crate,
with `LIBPNG_DEFINES` from `libs/libpng/configure.ac` (`PNG_NO_MMX_CODE`;
`PNG_CONFIGURE_LIBPNG` only makes `pngpriv.h` include configure's
`config.h`, whose checks every host we build on passes, so it is left out).
libpng's `zlib.h` is TeX Live's zlib (`third_party/zlib`) with the same
`Z_PREFIX`. On `aarch64`/`arm` hosts the `arm/` NEON filter files are
compiled, as TeX Live's `PNG_ARM_NEON` conditional does; elsewhere
`PNG_ARM_NEON_OPT=0`. The NEON code only speeds up unfiltering; its results
are the C code's.

libpng signals errors with `longjmp`, which must never cross Rust frames, so
every libpng call goes through `crates/flashtex-engine/csrc/png_shim.c`
(part of the GPL crate), which sets its own `setjmp` and returns an error
code.

## Licence

The PNG Reference Library License version 2 (`libpng-src/LICENSE`), a
permissive licence compatible with the GPL.
