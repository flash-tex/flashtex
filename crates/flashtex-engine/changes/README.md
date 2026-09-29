# `crates/flashtex-engine/changes/`: WEB change files

`third_party/pdftex/pdftex.web` is never edited. Everything this engine does
differently from it is a WEB change file here, applied by `tools/web2rust`
exactly as TANGLE applies one (`@x` lines must match the master; see
`tools/web2rust/src/changes.rs`). Several change files are applied one after
another, in the order `web2rust-default.args` lists them, as `tie -c`
combines web2c's change files for TeX Live's pdfTeX.

They are GPL-2.0-or-later, like the rest of the crate. Each change's `@x` line
names the line of `pdftex.web` it replaces and, where it re-specifies web2c,
the part of `tex.ch` or `pdftex.ch` it stands in for (DESIGN.md §4.1: "the
system-dependent behaviour web2c adds ... is re-specified by us").

| file | what it does |
|---|---|
| `web2c.ch` | What pdftex.web takes for granted from web2c: `orig_char_info`; the frozen-control-sequence layout that makes room for `\pdfprimitive`'s table (pdftex.ch); the run-time arrays web2c allocates; the date and time (`SOURCE_DATE_EPOCH`, `FORCE_SOURCE_DATE`); the expansion-depth limit; more than 256 fonts (`max_font_max`, 16-bit font numbers in char nodes, `fnt_def2`/`fnt2` in DVI); the newline before the program ends; `hash_size+hash_extra` in the statistics. |
| `goto.ch` | The one jump that enters a sibling `case` arm (`prune_page_top`), which labelled blocks and loops cannot express, rewritten into the same statements in the same order. |
| `precedence.ch` | Four expressions whose C reading (web2c prints Pascal's `and`/`or` as C's `&&`/`\|\|` with C's precedence) differs from their Pascal reading; the parentheses the C compiler implies are made explicit. `web2rust` refuses any such expression, so this list is complete. |
| `ext.ch` | The interface to pdfTeX's C parts (`pdftex.defines`): their routines declared `external` with Pascal types, bodies in `src/pdftex/`; `pdftex.h`'s macros as Pascal; the PDF buffer pointer `pdf_buf` as an explicit choice between the two buffers; kpathsea's PK set-up as one call. |

New sections are added only at the end of part 54 ("System-dependent
changes"), as tex.web asks, so the section numbers `// §NNNN` in
`src/generated/` stay those of pdftex.web up to part 54.

## Not re-specified yet

TeX Live's pdfTeX is pdftex.web plus web2c's `tex.ch` and a dozen further
change files (`pdftexdir/am/pdftex.am`). What they do beyond the table above
is not here yet; the parts visible in logs are the e-trip test's listed
normalisations (`scripts/flashtex-etrip.sh`) and P-T1's business:

- the primitives `\tracingstacklevels`, `\partokenname`, `\partokencontext`,
  `\showstream` (public-domain change files `tracingstacklevels.ch`,
  `partoken.ch`, `showstream.ch`) and `\synctex`;
- `\hyphenation` exceptions in a chained hash (`tex.ch` [42.934-941]);
- file names: quoting, `\input{...}`, `-output-directory`, the recorder,
  `\openout` lines in the log, `print_file_name` (`tex.ch` [29]);
- the log's status lines (` restricted \write18 enabled.`,
  ` %&-line parsing enabled.`), `\write18`, TCX character translation and
  the 8-bit printable range, `file:line:error` messages, source specials;
- `unbalanced-braces.ch`, `char-warning-pdftex.ch` (`\tracinglostchars>2`).
