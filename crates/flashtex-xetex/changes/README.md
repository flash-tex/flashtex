# `crates/flashtex-xetex/changes/`: WEB change files

`third_party/xetex/xetex.web` is never edited. Everything this port does
differently from it is a WEB change file, applied by `tools/web2rust` exactly
as TANGLE applies one, in the order `../web2rust-default.args` lists them
(which follows TeX Live's `xetexdir/am/xetex.am`). See
`docs/design/xetex/PLAN.md` for the plan and the results.

TeX Live builds XeTeX from `xetex.web` with `tex.ch`, TeX Live's feature
change files, SyncTeX's and `xetexdir/xetex.ch`. `tex.ch` and `xetex.ch` are
web2c's: they are written for a C translation (pointers, `xmalloc`, C
strings), so, as for the pdfTeX engine, their effect is re-specified here.
Most files are the pdfTeX engine's (`crates/flashtex-engine/changes/`), hunk
for hunk wherever `xetex.web` has the same text; the XeTeX differences are
listed in each file's header. The review method: the WEB these files produce
(`web2rust --emit-web`) was diffed against TeX Live's own merged WEB
(`tie` of `xetex.web`, `tex.ch0`, `tex.ch`, the feature files, SyncTeX's
and `xetex.ch`), and every difference that is not system-dependent was
either removed or is listed under "Not re-specified" below.

The files here are GPL-2.0-or-later, like the rest of the crate. TeX Live's
own public-domain feature change files are used from
`third_party/pdftex/web2c/` and `third_party/xetex/`, unmodified.

In order:

| file | what it does |
|---|---|
| `web2c.ch` | What xetex.web takes for granted from web2c (the pdfTeX engine's `web2c.ch`): `xchr` the identity of bytes (xetex.h's `Xchr`), no `xord`/`xprn`; `qi`/`qo`/`hi`/`ho` the identity (tex.ch [8.112]; XeTeX's `min_halfword` is negative); `save_arith_error`; more than 256 fonts; texmfmem.h's `b0`/`b1`; no `?.?` glue ratio; `hash_extra`; ML\TeX's `char_sub_code_base` (sized by `number_usvs`, xetex.ch); the date; the expansion-depth limit; `\noexpand\endwrite` refused (tex.ch [25.369]); `\mkern` recovery (tex.ch [26.449]); TFM name lengths and the message "Metric (TFM) file or installed font not found" (xetex.ch); fonts scaled to 2048pt; the DVI size limit and 16-bit font numbers in it; repeated `\hyphenation` words; bigtrie; `\font` reuse without overflow; `hash_high` in the format; the editor (`E`). |
| `goto.ch` | The one jump into the other branch of an `if` (`line_break`'s native-word hyphenation goes to `done3`), rewritten as a jump to a label of its own branch. |
| `precedence.ch` | Expressions whose C reading differs from Pascal's: three `and`/`or` next to a comparison, one negative `null` next to `and` (`is_glyph_node`), and two negated reals, which web2c truncates. `web2rust` refuses all of them, so the list is complete. |
| `ext.ch` | The interface to XeTeX's C parts (`xetex.defines`): every routine declared `external`, bodies in `src/xetex_ext.rs` and `src/system.rs`; C pointers as integer handles (`void_pointer`, `nil`, glyph-info arrays, picture paths); `addressof(x)` as `var` parameters; `sizeof`; the font arrays `xetex.ch` adds; the growable `native_text`, `mapped_text` and `xdv_buffer`. |
| `filenames.ch` | File names as tex.ch and xetex.ch treat them: UTF-8 in `name_of_file`, UTF-16 in the pool; `"` or `'` quoting; `/` areas and the last `.`; braced names; no forced `.tex`; `\input` and `\openin` open Unicode files (`u_open_in`) and re-parse the name found; `\openin`/`\input` check `openin_any`; string recycling over the pool strings only (above 65535); `\openout` logged. |
| `virtex.ch` | INITEX and production runs in one program, `-etex`, the default format (the pdfTeX engine's, unchanged). |
| `web2c-hooks.ch` | The text of tex.ch that TeX Live's feature change files match: web2c's integer parameters (before e-TeX's in XeTeX) and one undump line. |
| *third_party/pdftex/web2c/* | `tracingstacklevels.ch`, `partoken-102.ch`, `partoken.ch`, `locnull-optimize.ch`, `unbalanced-braces.ch`, `showstream.ch`, unmodified. |
| `synctex.ch` | SyncTeX's memory layout as `synctex-xe-def.ch0` and the other synctex files make it for XeTeX: the file tag and line in ONE word (`lh`, `rh`), so box nodes have 8 words, rule nodes 5, math/glue/kern/penalty nodes 3; the `\synctex` parameter; no `.synctex` file. |
| `xetex-web2c.ch` | What `xetex.ch` and `tex-binpool.ch` change in xetex.web itself: the banner; Unicode terminal and input files; the pool loaded from the program; `name_of_file` ends with a 0; `history=output_failure`; `new_character` for native fonts; `hlist_out`'s labels and an upwards empty box in `vlist_out`; `dvi_close`; bigtrie by `biggest_lang` and `max_hyph_char`; the format holds the pool strings' starts only; no native fonts in a format; a shorthand definition starts from `\relax`'s `too_big_usv` ([49.1222]); the code after the main program moved before it. |
| `mltex.ch` | tex.ch's ML\TeX character access, through which `xetex.ch` applies a TFM font's TECkit mapping: `char_info` reads the effective character (`effective_char`, which maps it unless it is a ligature's, `xtx_ligature_present`), `orig_char_info` where tex.ch avoids substitution (font loading, `new_character`, `hlist_out`'s output, math), and the main loop checks the effective character. ML\TeX's substitutions themselves are not re-specified (`mltex_enabled_p` is always false). |
| *third_party/xetex/* | `char-warning-xetex.ch`, unmodified. |
| `web2c-run.ch` | How a run is set up and reports itself (the pdfTeX engine's `web2c-run.ch`): texmf.cnf values, `-interaction`, file:line:error, `-halt-on-error`, the status lines (a TCX file is "ignored", as xetex.ch says), `%&format`, `-jobname`, `\write18` (the command made UTF-8), `openin_any`/`openout_any`, `-output-comment`, `-no-pdf`; no TCX tables in the format and no pdfTeX banner. |

## Not re-specified (phase S0)

What TeX Live's XeTeX does that this port does not do yet. None of it can
show in a document that uses TFM fonts only and none of the features named;
`docs/design/xetex/PLAN.md` gives the phase that brings each.

- Native (OpenType/AAT/Graphite) fonts: `find_native_font` finds none, so a
  quoted font name that TeX Live finds as an installed font is a TFM lookup
  here (phase S1); the OpenType math routines (S1-S2); `\XeTeXglyph` etc.
- `\XeTeXinputnormalization` (S1; TECkit's normalizer is
  `Globals::normalize_utf32`, not yet called from `input_line`).
- ML\TeX's character substitutions (`mltex.ch` keeps only the effective
  character's TFM font mapping). A mapped character outside the font's
  range read by `char_info` without a range check (`\fontcharwd` etc.)
  reads `font_info` beyond the character table in TeX Live's xetex, through
  texmfmem.h's little-endian `fmemoryword` union; this port's word packing
  differs there, so such a read gives other garbage (a checked index, so at
  worst a panic beyond `font_info`, where C reads out of bounds).
- ICU: `\XeTeXinputencoding` with a name other than XeTeX's built-in ones
  (`utf8`, `utf16`, `utf16be`, `utf16le`, `bytes`, `auto`) reads bytes, as
  TeX Live does for a name ICU does not know; ICU line breaking
  (`\XeTeXlinebreaklocale`) serves native fonts only (S1).
- Pictures: `\XeTeXpicfile` and `\XeTeXpdffile` find no file (S2).
- The output driver: `-no-pdf` is the only mode; without it the program
  still writes XDV (phase 3, with the PDF backend).
- tex.ch's hashed `\hyphenation` table (its order shows only in a format),
  ML\TeX (`-mltex`), encTeX, source specials, `-synctex`, IPC: as in the
  pdfTeX engine (`crates/flashtex-engine/changes/README.md`).
- `\XeTeXinputencoding` on the terminal level: `input_file[0]` is not the
  terminal here.
