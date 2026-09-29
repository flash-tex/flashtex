# `crates/flashtex-engine/changes/`: WEB change files

`third_party/pdftex/pdftex.web` is never edited. Everything this engine does
differently from it is a WEB change file, applied by `tools/web2rust` exactly
as TANGLE applies one (`@x` lines must match the master; see
`tools/web2rust/src/changes.rs`). Several change files are applied one after
another, in the order `web2rust-default.args` lists them, as `tie -c`
combines web2c's change files for TeX Live's pdfTeX.

The files here are GPL-2.0-or-later, like the rest of the crate. Each change's
`@x` line names the line of `pdftex.web` it replaces and, where it
re-specifies web2c, the part of `tex.ch` or `pdftex.ch` it stands in for
(DESIGN.md §4.1: "the system-dependent behaviour web2c adds ... is
re-specified by us"). TeX Live's own public-domain feature change files are in
`third_party/pdftex/web2c/` and are applied unmodified.

In order:

| file | what it does |
|---|---|
| `web2c.ch` | What pdftex.web takes for granted from web2c: `orig_char_info`; every character is itself, printable if ASCII (tex.ch [2]); the frozen-control-sequence layout that makes room for `\pdfprimitive`'s table (pdftex.ch); the run-time arrays web2c allocates; the date and time (`SOURCE_DATE_EPOCH`, `FORCE_SOURCE_DATE`); the expansion-depth limit; more than 256 fonts (`max_font_max`, 16-bit font numbers in char nodes, `fnt_def2`/`fnt2` in DVI); more than 255 hyphenation ops per language ("bigtrie"); the newline before the program ends; `hash_size+hash_extra` in the statistics. |
| `goto.ch` | The one jump that enters a sibling `case` arm (`prune_page_top`), which labelled blocks and loops cannot express, rewritten into the same statements in the same order. |
| `precedence.ch` | Four expressions whose C reading (web2c prints Pascal's `and`/`or` as C's `&&`/`\|\|` with C's precedence) differs from their Pascal reading; the parentheses the C compiler implies are made explicit. `web2rust` refuses any such expression, so this list is complete. |
| `ext.ch` | The interface to pdfTeX's C parts (`pdftex.defines`): their routines declared `external` with Pascal types, bodies in `src/pdftex/`; `pdftex.h`'s macros as Pascal; the PDF buffer pointer `pdf_buf` as an explicit choice between the two buffers; kpathsea's PK set-up as one call. |
| `filenames.ch` | File names as tex.ch [29] treats them: `/` areas, the last `.` starts the extension, `"` quoting, `\input{...}`, no forced `.tex` (the resolver tries `name.tex`, then `name`), the name found shown in the log, `\openout` logged when texmf.cnf's `log_openout` says so. |
| `virtex.ch` | INITEX and production runs in one program (tex.ch's `-ini`), `-etex` (pdftex.ch), and a production run's default format (`-fmt`, else the program name), which its banner names. |
| `web2c-hooks.ch` | The lines of tex.ch that TeX Live's feature change files are written against (web2c's integer parameters, which start with ML\TeX's three; one undump line). |
| *third_party/pdftex/web2c/* | `tracingstacklevels.ch`, `partoken-102.ch`, `partoken.ch`, `locnull-optimize.ch`, `showstream.ch`, `unbalanced-braces.ch`, unmodified. |
| `synctex.ch` | The `\synctex` parameter (SyncTeX itself is not here). |
| *third_party/pdftex/web2c/* | `char-warning-pdftex.ch`, unmodified. |

New sections are added only at the end of part 54 ("System-dependent
changes"), as tex.web asks, so the section numbers `// §NNNN` in
`src/generated/` stay those of pdftex.web up to part 54.

## Not re-specified yet

What else TeX Live's pdfTeX does that is visible, measured on
`\documentclass{article}...Hello` against TeX Live 2026's `pdflatex` (both
logs otherwise identical):

- the log's status lines ` restricted \write18 enabled.` and
  ` %&-line parsing enabled.`, and `\write18` itself (DESIGN.md §4.5 turns it
  off by default, which is a product decision for P-T1);
- TCX files (`-translate-file=cp227.tcx`, which makes characters 128–255
  printable in logs of documents run with pdflatex.fmt) and `-8bit`;
- the font map (`{.../pdftex.map}` in the log) and font embedding
  (`<.../cmr10.pfb>`): pdfTeX's C parts, P3;
- SyncTeX (its node fields show only in memory accounting);
- tex.ch's hashed `\hyphenation` exceptions (they show only in the counts);
- `-output-directory`, the recorder, `file:line:error` messages, source
  specials, the interaction options, `-jobname`.
