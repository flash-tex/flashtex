# `third_party/xpdf`

TeX Live 2026's own xpdf, **unmodified**, the PDF parser of the engine's PDF
inclusion (`crates/flashtex-engine/src/pdftex/pdftoepdf.rs`, the port of
pdfTeX's `pdftoepdf.cc`). pdfTeX reads included PDFs with this library: its
xref handling and repair, object streams, page-attribute inheritance, number
lexing and font encodings decide what pdfTeX copies, so the engine links the
same code at the same version. See
`docs/evidence/pdf-images-2026-09-29/README.md` for the decision.

## Pin

| | |
|---|---|
| Upstream | TeX Live source tree, GitHub mirror <https://github.com/TeX-Live/texlive-source> |
| Tag | `tags/texlive-2026.1` ("texlive-2026.1 tag based on r78399") |
| Commit | `6a300188053b8f2ded89dbd52293732a706b9c0e` (the same pin as `third_party/pdftex`) |
| Path | `libs/xpdf/xpdf-src/` |
| Version | xpdf 4.06 (`libs/xpdf/version.ac`); TeX Live 2026's `pdftex --version` says "Compiled with xpdf version 4.06" |
| Fetched | 2026-09-29 |

TeX Live's changes to the xpdf 4.06 release are in its tree already
(`libs/xpdf/TLpatches/`: `aconf.h.in` removed, `PDF_PARSER_ONLY` guards in
`gfile`, Windows file-name handling); the vendored files are TeX Live's.

## Files

`xpdf-src/` holds the sources of TeX Live's `libxpdf.a` (the `goo`, `fofi`
and `xpdf` source lists of `libs/xpdf/Makefile.am`), the headers they
include (found with `c++ -MM` over those sources; `splash/` contributes
`SplashTypes.h` only), and `COPYING`, `COPYING3`, `README` and `ANNOUNCE`.
The viewer, the tools and the rendering code (`splash/*.cc`) are not
vendored. Checksums of every file are in `SHA256SUMS`, and each file is
byte-identical to TeX Live's:

```sh
cd third_party/xpdf && shasum -a 256 -c SHA256SUMS
```

## Build

`crates/flashtex-engine/build.rs` compiles the sources with the `cc` crate
(C++), with TeX Live's `AM_CPPFLAGS` (`-DPDF_PARSER_ONLY`, include paths
`goo`, `fofi`, `splash`, `xpdf`) and `NO_WARN_CXXFLAGS`
(`-Wno-write-strings`). `aconf.h`, which TeX Live's `configure` generates,
is `crates/flashtex-engine/xpdf-config/aconf.h`: no A4 paper, no OPI, no
multithreading, no C++ exceptions (TeX Live's defaults), and the header and
function checks every host we build on passes.

The C interface the Rust port calls is
`crates/flashtex-engine/csrc/xpdf_shim.cc` (part of the GPL crate): one
wrapper per xpdf call pdftoepdf.cc makes, which also checks the object types
xpdf's accessors assume.

## Licence

xpdf is distributed under the GNU General Public License, version 2 or
version 3, and explicitly **not** "any later version" (`xpdf-src/README`,
"License & Distribution"; texts in `xpdf-src/COPYING` and
`xpdf-src/COPYING3`). That is compatible with the engine crate's
GPL-2.0-or-later sources (pdfTeX itself links it), but an engine binary that
links xpdf can be distributed under GPL v2 or GPL v3 only; the owner's legal
review of DESIGN.md §3 should note it. When the engine is shipped in binary
form, xpdf's README asks for its `README`, `COPYING` and `COPYING3` to be
included. It may only be linked into `crates/flashtex-engine` (DESIGN.md §3).
