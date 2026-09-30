# PDF backend, part 2: bitmap (PK/Type 3), TrueType, OpenType and subfont fonts (2026-09-30)

Lane **P3-FONTS-2** (kabir-claude, mac-m5pro-kabir), DESIGN.md §1.1 P-T2,
§6.1, §6.3, §4.1 "C parts". Branch `agent/kabir-claude/p3-fonts-2`, from
`origin/main` (02dcf9d07). Follows lane P3-FONTS
([`../pdf-backend-2026-09-29/`](../pdf-backend-2026-09-29/README.md)),
which ported pdfTeX's Type 1 path and stopped every other font with an
error.

Before this lane a document using a font pdfTeX writes as Type 3 (every
METAFONT-only font: bbm, Concrete, wasy's relatives, chess, IPA, Knuth's
punk, ...), a TrueType or OpenType map entry, or a TrueType subfont entry
stopped with "not implemented". Now each is written as pdfTeX writes it,
byte for byte, and reaches the preview's display list.

## Source of record

pdfTeX 1.40.29's C sources from the TeX Live source tree, the pin of
`third_party/pdftex` (tag `texlive-2026.1`), `texk/web2c/pdftexdir/`, read,
not vendored; each Rust function names its C original.

| file | sha256 |
|---|---|
| `writet3.c` | `51e7f76efae2ba86cf13d1f755e554040a2decec06fccf9c7ebe276b0772a6e5` |
| `pkin.c` | `dddc6ed975ed7f7e6d3d983159882f106e47d48f39a8f2a4146e3516f0601a07` |
| `writettf.c` | `39e2ebe6beb245120d6eb0c2fdf9c3030c1e5069d6f60be2f86d4604536002d0` |
| `writettf.h` | `3b3ade8b04953ca8e6e75a72cbcad4a873623e1ba41862f1254b23ecd9d181d5` |
| `subfont.c` | `960939e954da726dc59ded57aa9fdd68912c46983927b2bfa59a68a6e96742d9` |
| `macnames.c` | `f00e6feae27cf3a99f4b9cf99b712aae73cbf342e45202512b950c38fd2241c4` |

## Reuse before building (DESIGN.md §1): per file, port or link

The question is the one P3-FONTS answered for its files: can TeX Live's
unmodified C be linked into the GPL crate with no compromise? These files,
like the rest of `pdftexdir/`, are compiled by web2c against `pdftexd.h`
and read and write the engine's globals and call its routines
(`pdfcharmarked`, `pdffontsize`, `objptr`, `pdfnewdict`, `pdfbeginstream`,
`getcharwidth`, `dividescaled`, the font buffer `fb_array`, `fd_entry` and
`fm_entry` of the already-ported writefont/mapfile, `make_subset_tag`,
`pdftex_fail`); linking them needs the C ABI shim over the engine's Rust
globals that the rule forbids. So they are **ported**; the libraries they
call, which have no such coupling, are **linked**.

| C file | decision | Rust | why |
|---|---|---|---|
| `writet3.c` | port | `src/pdftex/writet3.rs` | engine globals and the PDF writer; `.pgc` and PK paths, Type 3 dictionary, `/Widths`, `/Encoding`, `/CharProcs`, ToUnicode |
| `pkin.c` | port | `writet3.rs` (`PkReader`) | reads through writet3's `t3_file` and fails through `pdftex_fail`; as a Rust reader with errors as values it also serves the display list, which must never stop the run |
| `writettf.c` | port | `src/pdftex/writettf.rs` | `fd_entry`, the font buffer (`fb_putchar`, `fb_seek`, the checksum over `fb_array`), `make_subset_tag`, writefont's descriptor; TrueType subsetting and whole fonts, OpenType (CFF) embedding |
| `subfont.c` | port | `src/pdftex/subfont.rs` | `fm_entry`/`avl_do_entry` of the ported mapfile, `open_input`, `tex_printf` |
| `macnames.c` | port (tables) | `src/pdftex/macnames.rs` | two string tables, generated from the file |
| kpathsea `tex-glyph.c`, `magstep.c`, `proginit.c`, `tex-make.c` | **link** (vendored unmodified, `third_party/kpathsea`) | `kpathsea-config/flashtex_kpse.c`, `resolver.rs` | `kpse_find_pk`, `kpse_magstep_fix`, `kpse_bitmap_tolerance`, `kpse_init_prog`, and mktexpk through `kpathsea_make_tex`: the library pdfTeX calls |
| `mktexpk`, METAFONT, `gftopk` | TeX Live's programs, run by kpathsea | | as for pdftex, out of process |

Evaluated and not used: Rust TrueType crates (`ttf-parser`, `allsorts`,
`subsetter`) subset fonts, but not into pdfTeX's bytes (table order, the
`name` table's subset tags, `OS/2` rewritten to version 1, `post` format
2/3 choice, `cmap` (1,0)+(3,0), `loca` format from the new offsets,
`checkSumAdjustment` over the whole buffer): identical font subsets are
P-T2's measure, so they fail the "no compromise" test. No PK reader crate
exists that reproduces pkin.c's unpacking, and the bits must be those
pdfTeX puts in the PDF.

### mktexpk, `\write18` and kpathsea

pdfTeX does not run mktexpk through `\write18`: kpathsea runs it
(`kpathsea_make_tex`) when `kpse_find_pk` misses and the `pk` format's
program is enabled. pdftex.web enables it at the lowest level
(`kpse_set_program_enabled(kpse_pk_format, 1, kpse_src_compile)`) when PDF
output starts, after `kpse_init_prog('PDFTEX', \pdfpkresolution,
\pdfpkmode, nil)` has set `MAKETEX_BASE_DPI` and `MAKETEX_MODE`; texmf.cnf's
or the environment's `MKTEXPK=0` still turns it off. The engine does
exactly this with the linked kpathsea (`pk_init` → `flashtex_kpse_init_pk`),
so restricted shell escape has no bearing on it, as in pdflatex. The
kpathsea instances that must not run scripts (kpsewhich-compatible lookups,
the bundle) keep it disabled at command-line level, as before. Every PK
file mktexpk makes is recorded as an external effect (`mktex`), an L3
barrier for the incremental system (DESIGN.md §4.5). The display-list
writer looks PK files up with mktexpk disabled.

## Verified results

Oracle: TeX Live 2026 `pdftex` 1.40.29 (`pdfTeX 3.141592653-2.6-1.40.29 (TeX
Live 2026)`), on the NixOS PC (x86_64, 16 threads, `~/texlive/2026`,
scheme-full), `SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1`. Engine: this
branch merged locally with PR #1232 (the NixOS rpath fix). All numbers
below are from that machine; the Mac (MacTeX) was not used for these runs.

### Byte-identical PDFs: `crates/flashtex-engine/tests/pdf_fonts2.rs`

13 plain TeX documents, each engine with its own plain.fmt and its own empty
`TEXMFVAR` (so mktexpk runs in both), `\pdfsuppressptexinfo=-1`: the PDFs
must be byte-identical and the backend log lines (`{map}`, `{enc}`,
`{sfd}`, `<font file>`, `Output written`) the same, the `TEXMFVAR` roots
compared as one. **13/13 identical.**

| case | what |
|---|---|
| `pk-bbm` | bbm10 (METAFONT only), made by mktexpk, characters 0 and 255 too |
| `pk-sizes` | `\magstep2`, `scaled 1440`, `at 7.3pt` (magstep fix, bitmap tolerance), bbm12 |
| `pk-300dpi` | `\pdfpkresolution=300` |
| `pk-mode` | `\pdfpkresolution=300 \pdfpkmode={cx}` (mktexpk `--mfmode cx`) |
| `pk-cm` | cmr10/cmmi10/cmsy10/cmex10 with their map entries deleted: Computer Modern as Type 3 |
| `pk-enc` | a bitmap map entry with an encoding (`=bbm10 <8r.enc`): glyph names, ToUnicode |
| `pk-attr` | `\pdffontattr` on a Type 3 font, uncompressed |
| `pgc` | a `.pgc` Type 3 glyph file (pdfTeX's own format) |
| `ttf-arvo` | Arvo (TrueType, `pdftex.map`): subset, reencoded, directly and through a virtual font |
| `ttf-whole` | `<<Arvo-Regular.ttf`: the whole font, no encoding |
| `ttf-tounicode` | Clear Sans with `\pdfgentounicode=1` |
| `otf-bodoni` | GFS Bodoni (OpenType CFF, `pdftex.map`) |
| `ttf-subfont` | `\pdfmapline{+arvou@Unicode@ <Arvo-Regular.ttf}` with TFMs made by ttf2tfm and TeX Live's `Unicode.sfd`: subfonts 00 and 20 |

A second test in the file writes the display list for `pk-bbm`,
`pk-sizes`, `ttf-arvo` and `otf-bodoni` and checks that the PDF does not
change when a display list is written, that the `type3` program decodes
and has a bitmap for every glyph the pages use, that the TrueType and
OpenType programs are the font files, and that no page is flagged
incomplete: passes.

### Font-family census (P-T2 on every font family)

`tools/font-census/census.py`: every directory of TFM files under
`texmf-dist/fonts/tfm` is a family; up to 6 of its fonts (spread over the
list) are typeset testfont-style (all 256 characters at design size) by
plain TeX with both programs, and the PDFs compared byte for byte (which
includes the embedded font subsets, P-T2's measure) with the backend log
lines. A family's kind is how pdfTeX writes most of its fonts.

`census-families.json` / `census-families.md` (2276 fonts, 71 s):

| kind | families tested | identical | pdfTeX itself stops (engine stops the same way) | failing |
|---|---|---|---|---|
| type1 | 298 | 294 | 4 | 0 |
| vf (virtual) | 83 | 74 | 9 | 0 |
| pk (METAFONT only → Type 3) | 71 | 70 | 1 | 0 |
| truetype | 3 | 3 | 0 | 0 |
| opentype | 3 | 0 | 3 | 0 |
| **all** | **458** | **441** | **17** | **0** |

**458 families tested, 441 identical, 0 failing.** The 17 that neither
program writes stop with the same pdfTeX error in both: a map entry whose
font file TeX Live does not ship (Charter ITC, Adobe Garamond Pro, Utopia
Std OpenType; `ae_AlMohanad_xxbold.pfb`, `Kerkis-SemiBoldItalic.pfb`,
`putc8a.pfb`, `ugmm8a.pfb`, `uagb8a.pfb`: "cannot open ... font file for
reading"), or a font with neither a map entry nor a METAFONT source
(`ari10u`, `cmgmiu8r`, `jtmro8rw`, `bchbc8a`, `r-cfjas-x-t1`, `tnganai`,
`wnrit8`, `ubkro8r`, `uhvro8rn`: "Font ... at 600 not found"). The OpenType
fonts TeX Live does ship (GFS Bodoni and Didot) are used by the
`mathdesign/mdgreek` family, counted under another kind because most of its
fonts are not OpenType: identical, OpenType members included.

Then **every** font of the families with bitmap or TrueType fonts
(`--per-family 100000 --kind pk --kind truetype --kind opentype`,
`census-bitmap-truetype-all.json`, 1509 fonts, 657 s, mktexpk making most
PK files): 71 of 74 families identical at once; `jknappen/fc` (204 fonts)
and `public/drm` (334 fonts) differed only because the oracle, making
their PK files one after another, hit the census's 300 s timeout and left
a partial PDF; rerun with the PK files made
(`census-bitmap-rerun-fc-drm.json`): both identical (16.7 MB and 18.0 MB
PDFs, byte for byte). `public/wnri` stops in both (no METAFONT source). So
**1313 PK fonts and 62 TrueType fonts written identically**, 11 fonts that
pdfTeX cannot write either.

The METAFONT-only families tested (all 71, identical but `wnri`, which no
program can write): jknappen/fc, amsfonts/dummy, astro, bahaistar, bangtex,
barcodes, bartel-chess-fonts, bbm, bengali, boisik, bookhands, casyl,
cbcoptic, cherokee, chess, circ, clock, cm-mf-extra-bold, cmextra (its
METAFONT-only members), cmpica, concmath-fonts, concrete, cookingsymbols,
cryst, cs, ctib, dancers, dice, dingbat, dozenal, drm, duerer, eiad, elvish,
etex, euro-ce, euxm, feyn, genealogy, go, gothic, greenpoint, hands, ibygrk,
ifsym, kixfont, knuth-local, levy, lfb, montex, nkarta, obnov, ogham,
oldlatin, orkhun, othello, otibet, pacioli, phonetic, punk, ruscap,
schulschriften, shuffle, skak, skull, trsym, universa, wnri, wsuipa, xq,
yannisgr. Those seen on arXiv and CTAN documents include bbm, concrete and
concmath (Concrete Mathematics), euxm, phonetic and wsuipa (IPA), skak and
chess, feyn (Feynman diagrams), cryst, go, dancers, dice, ifsym, trsym,
cmpica, punk, cs (Czech), yannisgr/ibygrk/levy (Greek), bengali, ogham,
cbcoptic, xq.

### Display list (DESIGN.md §6.1)

`docs/protocol/display-list-v3.md` §5.1, §5.1.1, §6.3, §9, §10, additive
(version stays 3.1: new JSON keys and values of a key the spec had
reserved):

* **`type3`**: the `FONT` program is the font's glyphs as bitmaps (§5.1.1,
  `"T3B1"`: per character its image mask, placed as pdfTeX's glyph
  procedure places it, the rows bit for bit those writepk puts in the PDF),
  with `font_matrix` (pdfTeX's Type 3 `/FontMatrix`, as it prints it) and
  `dpi`. The MIT crate encodes and decodes it
  (`flashtex_display_list::resource::Type3Bitmaps`); the engine fills it
  from the ported PK reader.
* **`truetype`**, **`opentype`**: the program is the font file; a subfont
  also carries `subfont` (its 256 character codes) and `cmap`.
* **Capability-gated**: the host advertises `font-formats`; programs of
  these three formats go only to a client whose `COMPILE` lists them in
  `font_formats`. Another client gets the `FONT` with an empty program (as
  if held), so a 3.1 app is unaffected. Type 1 fonts and every page item
  encoding are unchanged.
* **Positions**: the writer advances Type 3 glyphs by the Type 3 widths
  through the Type 3 `/FontMatrix` (writet3.c's `pk_char_width` and
  `pk_font_scale`, exact); `problem` flags the pages of a font the display
  list cannot draw (a PK file mktexpk has not made yet, on a document's
  first compile; a `.pgc` font) INCOMPLETE.
* `tools/displaylist/check_positions.py` (positions against pdflatex's
  PDF, 0 sp) now reads Type 3 widths through `/FontMatrix` and takes
  `--root`: on this lane's LaTeX documents
  ([`dl-docs/`](dl-docs/): bbm, Computer Modern as Type 3, Clear Sans
  TrueType, GFS Bodoni OpenType) **4/4 documents exact, 469 glyphs**.

**What the app must do (lane P3-APP-V3, PR #1254):** send
`"font_formats": ["type3", "truetype", "opentype"]` in `COMPILE` once it
draws them. `type3`: decode §5.1.1 and draw each glyph's mask as an image
mask in the fill colour with [`width` 0 0 `height` `llx` `lly`] ×
`font_matrix` × the glyph matrix (what Core Graphics does with the PDF's
inline image). `truetype`/`opentype`: load the program's bytes with Core
Text (`CTFontManagerCreateFontDescriptorFromData`), draw the glyph named
`encoding[code]` (`CGFontGetGlyphWithGlyphName`; `uniXXXX` and `indexN` as
the spec says), for a subfont the glyph the `cmap` subtable gives
`subfont[code]`. Draw pages whose fonts carry `problem` from `DONE.pdf`.

## Gates

Run on the NixOS PC at `402d77eed` merged locally with PR #1232
(`e517dc4f7`), by [`gates/run-gates.sh`](gates/run-gates.sh); raw output in
[`gates/`](gates/). The machine was shared with other lanes' jobs (load
average 15-37), so no timing here is a measurement.

| gate | result |
|---|---|
| P-T1 / P-T2, parity fixtures (`tools/parity`, `--pt on`) | **83/83, 83/83** (unchanged from main); accounting 83/83 differ, as on main (non-gating) |
| the same with a display list written | **83/83, 83/83** |
| display-list positions vs pdflatex's PDF (0 sp) | fixtures **83/83** (118,899 glyphs, 1,513 rules); this lane's font documents **4/4** (469 glyphs) |
| lockstep (`tools/lockstep`) | **260/260** equal, accounting 0 differ |
| trip / etrip | pass / pass (0 lines differ) |
| drift (`web2rust --test drift`) | pass |
| `pdf_fonts2` (13 byte-identity cases + display list), `pdf_backend`, `pdf_images`, `display_list_host` | pass |
| engine lib tests, display-list crate tests | pass (40 + 1 ignored; all) |
| `scripts/gate.sh pr` (on the PC) | tests (changed crates, 641 s), licence boundary, parity self-tests, bundled inventory: PASS; rustfmt and clippy are not installed on the PC (the rustfmt step "fails" on every file for that reason), the fixtures baseline step skips on Linux |
| rustfmt, `cargo clippy --all-targets -D warnings` (flashtex-engine, flashtex-display-list), licence boundary, parity self-tests | on macOS at the tip: clean |

The tip (`61e2ad700`, clippy fixes only) was rebuilt on the PC and
re-checked: the four test files above, the lib and display-list crate
tests, and the positions checker (83/83 fixtures, 4/4 font documents) pass.

## What is not covered

* A `.ttc` collection in a map entry (the code path is ported: the first
  font of the collection; TeX Live's `pdftex.map` has none, so no test).
* `.pgc` fonts in the display list (flagged INCOMPLETE; they exist only in
  pdfTeX's own tests).
* In the resident host, a page first sent with a PK font's `problem` is not
  re-sent when a later compile finds the PK file, unless the page changes:
  it stays drawn from the PDF, which is correct but slower.
* The Mac oracle (MacTeX) was not run for this lane; the tests skip where
  a case's fonts are not installed.

## Reproduce

```sh
cargo test --release -p flashtex-engine --test pdf_fonts2   # needs TeX Live
python3 tools/font-census/census.py --engine target/release/flashtex-initex \
    --texbin ~/texlive/2026/bin/x86_64-linux --out /tmp/census -j 6
python3 tools/displaylist/check_positions.py --engine target/release/flashtex-initex \
    --formats <dir with the engine's pdflatex.fmt> --oracle ~/texlive/2026/bin/x86_64-linux/pdftex \
    --root docs/evidence/pdf-fonts-2-2026-09-30/dl-docs
```
