# PDF backend: pdfTeX's font backend ported, TeX Live's zlib linked (2026-09-29)

Lane **P3-FONTS** (kabir-claude, mac-m5pro-kabir), DESIGN.md §6.3, §1.1 P-T2,
§4.1 "C parts", §12 P3. Branch `agent/kabir-claude/p3-fonts`, from
`agent/kabir-claude/p2-pdftex-core` (PR #1194).

The engine (`crates/flashtex-engine`, GPL-2.0-or-later) now writes real PDFs
for Type 1 text documents. Before this lane, `src/pdftex/` answered every
font as "no map entry" and dropped the PDF bytes.

## Source of record

pdfTeX 1.40.29's C sources from the TeX Live source tree, the same pin as
`third_party/pdftex` (tag `texlive-2026.1`, commit
`6a300188053b8f2ded89dbd52293732a706b9c0e`), path `texk/web2c/pdftexdir/`.
They were read, not vendored; each Rust function names its C original.

| file | sha256 |
|---|---|
| `mapfile.c` | `349e3078008137395ed59a9c4694c2708a0cc5fcceea4263bcd46f6f8b20b051` |
| `writefont.c` | `12ed0bf2c5f7637e5a93ea33aaa6e2ee55c8de647777a2b903185555f1103445` |
| `writet1.c` | `cb571e8a79c84988c0e6974c0fb8b92e035bf8c4cefe6c6c58cdb16f7999feb0` |
| `writeenc.c` | `fb8b203b2431eee2eafec0c6bb814b40fc207d4bdcdd3ae5b4c80ceee805d0c2` |
| `tounicode.c` | `b5077fc15119ef7003e3e4450c275acc58117eb5149fa2f3f3398c1181c4140f` |
| `writezip.c` | `d06fac015833c4d985339e427b9fc8477a2a92d447476ae98be790f5dbe85d61` |
| `utils.c` | `f289d4e42ed7f4128556ee2ccd674e9f6ad01c2b86c92749cc626287cbf759bc` |
| `subfont.c` | `960939e954da726dc59ded57aa9fdd68912c46983927b2bfa59a68a6e96742d9` |
| `writet3.c` | `51e7f76efae2ba86cf13d1f755e554040a2decec06fccf9c7ebe276b0772a6e5` |
| `ptexmac.h` | `4731e1a51cf3ae1fad935a9dab3b62c9c70caf019eee2551145b43f67e6237a1` |
| `ptexlib.h` | `e621a741d149d6f2673a311909759759ae2db5ad3399cc933ead78da21006506` |
| `pdftex.h` | `7bf602fc8f836ced1efaa6d113b87cb0667ee29ec615db0962ef1f9c88a2b5da` |

## Reuse before building (DESIGN.md §1): per file, port or link

The question per file: can TeX Live's unmodified C be linked into the GPL
crate with no compromise?

pdfTeX's `pdftexdir/*.c` are not a library. web2c compiles them against
`pdftexd.h`, the C translation of `pdftex.web`, and they read and write the
engine's globals directly (`pdfbuf`, `pdfptr`, `pdfgone`, `strpool`,
`poolptr`, `fontbc`, `pdfcharused`, `pdffontmap`, `objtab`, ...) and call
its routines (`pdfbeginobj`, `pdfbegindict`, `pdfnewobjnum`, `dividescaled`,
`getcharwidth`, `maketexstring`, `print`, `pdfflush`, ...). Our engine's
globals are fields of a Rust struct (`Globals`), so linking these files would
need a C ABI shim that exposes every such global by pointer and every callback
by trampoline, rewritten whenever the translation's layout changes, plus
web2c's `open_input`/kpathsea glue. That is the compromise the rule forbids,
so these files are **ported**; the libraries they call, which have no such
coupling, are **linked**.

| C file | decision | Rust module | why |
|---|---|---|---|
| `mapfile.c` | port | `src/pdftex/mapfile.rs` | engine globals (`pdffontmap`, `fontname`, string pool) |
| `writefont.c` | port | `writefont.rs` | calls `pdfbegindict`/`pdfnewobjnum`/`getcharwidth` and reads `pdfcharused` |
| `writet1.c` | port | `writet1.rs` | writes the engine's font buffer; calls `make_subset_tag`, `pdftex_warn` |
| `writeenc.c` | port | `writeenc.rs` | PDF object writer callbacks |
| `tounicode.c` | port | `tounicode.rs` | PDF writer callbacks; (un)dumps into the format file |
| `writezip.c` | port | `output.rs` | 60 lines around zlib that write `pdfgone`, `pdfstreamlength`, `pdflastbyte` |
| `utils.c` (output part) | port | `output.rs` | `fb_*`, `pdf_puts`/`pdf_printf`, `writestreamlength`, `printID`, `make_subset_tag`, `removepdffile`: all on engine globals |
| `pdftex.h`'s `writepdf` | port | `output.rs` | a macro over `pdfbuf` |
| `subfont.c` | port of the entry test only | `mapfile.rs` (`handle_subfont_fm`) | subfont (`name@sfd@`) entries serve TrueType only; they stop the run |
| `avl.c` (libavl) | replaced | `BTreeMap`/`BTreeSet` | a container, not behaviour: each tree becomes a map ordered by the tree's own comparison (`strcmp` is byte order), so every traversal visits entries in the C order |
| `md5.c` | already ported | `md5.rs` (P2) | |
| **zlib** (`libs/zlib`) | **link, unmodified** | `third_party/zlib`, `zlib.rs` | DESIGN.md §6.3; see below |
| C library `sscanf`/`snprintf` | **link** (the platform's) | `cfmt.rs` | pdfTeX parses and prints floats with them (`%f`, `%g`, `%i`, `%lX`); calling the same functions gives the same bytes in every edge case, where Rust's float parsing and formatting would not |
| `writet3.c`, `pkin.c` | not ported | `writet3.rs` | PK bitmap (Type 3) fonts: no Type 1 fixture uses them; such a font stops the run with pdfTeX's error layout |
| `writettf.c` | not ported | | TrueType/OpenType embedding: not needed by the fixtures; stops the run |

### zlib

TeX Live 2026's own zlib 1.3.2 is vendored byte-for-byte under
`third_party/zlib/zlib-src/` (sha256 of every file in
`third_party/zlib/SHA256SUMS`; pin in `third_party/zlib/README.md`) and
compiled by `crates/flashtex-engine/build.rs` with the `cc` crate. TeX Live
2026's `pdftex --version` reports "Compiled with zlib 1.3.2; using zlib
1.3.2", and `zlib::version()` returns "1.3.2" (unit test). Never miniz, the
system libz or a fork. `Z_PREFIX` (zconf.h's own switch) renames the symbols
to `z_*` so that no system libz in the same process can shadow them; the
`gz*.c` file layer is vendored but not compiled (the engine never calls it,
and it needs `unistd.h` declarations TeX Live's build gets implicitly).

## Verified results (measured on mac-m5pro-kabir, 2026-09-29)

Oracle: TeX Live 2026 `pdftex` 1.40.29 (`/Library/TeX/texbin/pdftex`),
`SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1`. Engine: this branch's
`flashtex-initex` (release), with a `pdflatex.fmt` it built itself from TeX
Live's `pdflatex.ini`. qpdf 12.4.2.

### P-T2 on the parity fixtures

`python3 tools/parity/parity.py --tier fixtures --engine <engine> --pt pt2 --raster none`
(the engine run through a test wrapper that maps pdfTeX's
`-interaction`/`-jobname` options, which the CLI lane adds to `main.rs`):

**P-T2 75/82 fixtures; 75/75 of those that qualify.** The scoreboard of that
run is `parity-fixtures-pt2.md` (and `.json`) beside this file.

Qualifying = uses only Type 1 fonts and no image or PDF inclusion. The 7 that
do not qualify are the beamer fixtures (`beamer-blocks-columns`,
`beamer-default`, `beamer-fragile`, `beamer-madrid`, `beamer-overlays`,
`beamer-polish`, `beamer-visuals`): beamer's inner theme loads
`beamericonbook.pdf` and friends with `\pdfximage`, which is PDF inclusion
(the images lane); the engine stops there with "image inclusion is not
implemented yet". Every other fixture of `fixtures/real-world` and
`fixtures/divergence-probes` passes: identical embedded font subsets
(program hashes), and per page identical content streams, resources (font
dictionaries, descriptors with `/CharSet`, `/Widths`, `/Encoding`,
`/ToUnicode`) and media boxes.

### Beyond P-T2: byte-identical PDFs

With `\pdfsuppressptexinfo=-1` (which leaves out `/PTEX.Fullbanner`), the
engine's PDFs are **byte-identical** to pdfTeX's, compressed object streams,
`/ID` and cross-reference table included, on every case tried:

* `crates/flashtex-engine/tests/pdf_backend.rs` (runs in `cargo test` where
  TeX Live is installed): plain TeX in PDF mode, 6 documents: Computer Modern
  text and math; a reencoded font (`ec-lmr10`, `.enc` file); `SlantFont` and
  `ExtendFont` via `\pdfmapline`; whole-font embedding (`<<cmr10.pfb`);
  `\pdfgentounicode=1` with `glyphtounicode.tex`; uncompressed streams. All 6
  identical, and their backend log lines too.
* A LaTeX sweep (20 documents, not committed): Bookman with its map file's
  `.167 SlantFont` entries, negative slant, extend, whole-font embedding (OT1
  and T1), non-embedded and standard-14 fonts (`/Flags` warning path), a
  `.pfa` font (pigpen), ToUnicode with T1/OT1/math/AMS fonts, compression
  levels 0 and 1, Times (`mathptmx`, 8r.enc), Latin Modern T1, Zapf
  Dingbats, `\pdfomitcharset=1`, `\pdffontattr`, `\pdfmapfile{=lm.map}`,
  duplicate/replace/delete/invalid `\pdfmapline`s, a multi-page document.
  All 18 that pdfTeX compiles produced byte-identical PDFs; the two where
  pdfTeX itself stops (`<<` embedding and non-embedded CMR10 under LaTeX's default
  `\pdfgentounicode=1`: "builtin glyph names is empty") stop the engine with
  the same error.
* Without `\pdfsuppressptexinfo`, `twelvept-plain` differs from pdflatex's
  PDF only by the missing `/PTEX.Fullbanner` entry.

### Backend log lines

`{.../pdftex.map}`, `{.../*.enc}`, `<.../*.pfb>` (and `<<...pfb>>` for whole
fonts) and `Output written on main.pdf (N pages, B bytes).` match pdflatex's
exactly, byte count included when the banner is suppressed. pdfTeX's
warnings (duplicate map entries, unknown map-line keys, non-embedded fonts,
`/Flags`) match word for word except the program name in them (next
section).

## Remaining differences, by cause

1. **`/PTEX.Fullbanner`**: pdfTeX writes web2c's banner there ("This is
   pdfTeX, Version ... (TeX Live 2026) kpathsea version 6.4.2"); the engine
   has no `pdftex_banner` string yet (it is set by web2c's startup code, the
   CLI/system lane). It is in the document information dictionary, outside
   P-T2, and DESIGN.md §1.1 says not to forge banners, so it stays out.
2. **Program name in warnings**: `kpse_invocation_name`. TeX Live prints
   argv[0] as invoked (`/Library/TeX/texbin/pdftex` in the harness), the
   engine prints its `-progname` (`pdflatex`). Owned by `system.rs`.
3. **Not ported** (each stops the run with an error, never a wrong PDF):
   PK/Type 3 fonts (`writet3.c`, `pkin.c`), TrueType/OpenType embedding
   (`writettf.c`), TrueType subfont map entries (`subfont.c`), and image/PDF
   inclusion (`writeimg.c`, `writepng.c`, `writejpg.c`, `writejbig2.c`,
   `pdftoepdf.cc`, `epdf.c`).

## What the images / PDF-inclusion lane can build on

* **Ready:** zlib with inflate compiled (`inflate.c`, `inftrees.c`,
  `inffast.c` are in the static library; only `deflate*` is bound in
  `zlib.rs` so far), `write_zip`, the font buffer (`fb_putchar`, `fb_flush`),
  `pdf_puts`/`pdf_printf`, `ByteFile::write_bytes`/`seek_to` in `system.rs`.
* **For PDF inclusion's font handling** (`epdf.c`): `lookup_fontmap`
  (ported in `mapfile.rs`), `epdf_write_enc` (ported in `writeenc.rs`),
  `register_fd_entry`/`lookup_fd_entry` (in `writefont.rs`), and `writet1`
  with `fd.all_glyphs`; still to port: `epdf_create_fontdescriptor`,
  `epdf_mark_glyphs`, `embed_whole_font`, `get_fd_objnum`, `get_fn_objnum`.
* **Still stubs:** everything in `src/pdftex/images.rs` (`read_image` fails),
  `dumpimagemeta`/`undumpimagemeta`. The lane decides, per §1, between
  porting and linking TeX Live's libpng and xpdf (the latter is C++ and
  coupled to `pdftoepdf.cc`'s use of engine globals, the same question as
  above).

## Reproduce

```sh
CARGO_BUILD_JOBS=4 cargo build --release -p flashtex-engine
cargo test -p flashtex-engine --test pdf_backend     # needs TeX Live
# P-T2: build pdflatex.fmt with the engine, then run tools/parity with an
# --engine that accepts pdfTeX's command line (see the report above).
```
