# `third_party/zlib`

TeX Live 2026's own zlib, **unmodified**, for the engine's PDF writer
(`docs/design/engine-v2/DESIGN.md` §6.3: "Use TeX Live's own zlib version,
statically linked (never miniz, the system libz, or a fork)"). pdfTeX's
`writezip.c` compresses every PDF stream with it, so the same library at the
same version gives the same compressed bytes.

## Pin

| | |
|---|---|
| Upstream | TeX Live source tree, GitHub mirror <https://github.com/TeX-Live/texlive-source> |
| Tag | `tags/texlive-2026.1` ("texlive-2026.1 tag based on r78399") |
| Commit | `6a300188053b8f2ded89dbd52293732a706b9c0e` (the same pin as `third_party/pdftex`) |
| Path | `libs/zlib/zlib-src/` |
| Version | zlib 1.3.2 (`libs/zlib/version.ac`); TeX Live 2026's `pdftex --version` says "Compiled with zlib 1.3.2; using zlib 1.3.2" |
| Fetched | 2026-09-29 |

TeX Live's only patch to this tree (`libs/zlib/TLpatches/zlib-caddr-fix.patch`)
touches `test/minigzip.c`, which is not part of the library and is not
vendored.

## Files

`zlib-src/` holds exactly the files of TeX Live's `libz.a`
(`nodist_libz_a_SOURCES` in `libs/zlib/Makefile.am`), plus `zlib.h`,
`zconf.h.in`, `LICENSE` and `README`. Checksums of every file are in
`SHA256SUMS`:

```sh
cd third_party/zlib && shasum -a 256 -c SHA256SUMS
```

## Build

`crates/flashtex-engine/build.rs` compiles the sources with the `cc` crate,
as TeX Live's `libs/zlib` does, except `gz*.c`: zlib's gzip file layer,
which the engine never calls (they stay vendored, unmodified, so the tree is
TeX Live's whole `libz.a` source set):

* `zconf.h` is generated from `zconf.h.in` into `$OUT_DIR`. TeX Live's
  `configure` runs `sed "/^#ifdef HAVE_UNISTD_H.* may be/s/.../"` over it,
  which matches no line of zlib 1.3.2's `zconf.h.in` (it says
  `#if HAVE_UNISTD_H-0`), so the generated file is a copy; ours is too.
* `ZLIB_DEFINES` (`-DUSE_MMAP`, and `_LARGEFILE64_SOURCE` where
  `KPSE_LARGEFILE` asks for it) are left out: no vendored file uses
  `USE_MMAP`, and 64-bit hosts have 64-bit `off_t` already. Neither affects
  `deflate`.

## Licence

The zlib licence (`zlib-src/LICENSE`), which is GPL-compatible.
