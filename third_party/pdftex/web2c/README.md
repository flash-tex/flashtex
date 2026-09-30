# `third_party/pdftex/web2c`: TeX Live's feature change files

TeX Live's pdfTeX is `pdftex.web` plus a list of change files
(`texk/web2c/pdftexdir/am/pdftex.am`, `pdftex_ch_srcs`). Besides web2c's
system layer (`tex.ch`, `pdftex.ch`, which `crates/flashtex-engine/changes/`
re-specifies), that list has small change files that add TeX features. These
are public domain and are used here **unmodified**, applied by
`tools/web2rust` in TeX Live's order, after
`crates/flashtex-engine/changes/web2c-hooks.ch`, which reproduces the lines of
`tex.ch` they are written against.

Same pin as `../README.md`: TeX Live source tree tag `tags/texlive-2026.1`,
commit `6a300188053b8f2ded89dbd52293732a706b9c0e`, fetched 2026-09-29.
Checksums are in `../SHA256SUMS`.

| file | upstream path | adds | licence (from the file) |
|---|---|---|---|
| `tracingstacklevels.ch` | `texk/web2c/tracingstacklevels.ch` | `\tracingstacklevels` | public domain (Petr Olšák, Akira Kakuto) |
| `partoken-102.ch` | `texk/web2c/partoken-102.ch` | the command code of `\partokenname` | public domain |
| `partoken.ch` | `texk/web2c/partoken.ch` | `\partokenname`, `\partokencontext` | public domain (Petr Olšák) |
| `locnull-optimize.ch` | `texk/web2c/locnull-optimize.ch` | `loc=null` implies `state=token_list` (no change in behaviour) | public domain (David Fuchs) |
| `showstream.ch` | `texk/web2c/showstream.ch` | `\showstream` | public domain (Marcel Krüger) |
| `unbalanced-braces.ch` | `texk/web2c/unbalanced-braces.ch` | fatal errors for over- or under-running `\output` and `\write` | public domain (David Fuchs) |
| `char-warning-pdftex.ch` | `texk/web2c/pdftexdir/char-warning-pdftex.ch` | `\tracinglostchars>2` makes a missing character an error | public domain (David Jones) |

Not taken from the list: `zlib-fmt.ch` (compressed formats), the three
encTeX files (only active with `-enc`), the SyncTeX files (see
`crates/flashtex-engine/changes/synctex.ch`) and `tex-binpool.ch` (the
string pool compiled into the binary).
