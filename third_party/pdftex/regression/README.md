# `third_party/pdftex/regression`

pdfTeX's own regression tests from the TeX Live 2026 source, unmodified,
with the files they read, at the same pin as `third_party/pdftex`. They are
DESIGN.md §8's tier T0 ("pdfTeX's regression directories") for the engine in
`crates/flashtex-engine`; `scripts/pdftex-regression.sh` runs them, and the CI
job `pdftex-regression` runs that script.

## Pin

| | |
|---|---|
| Upstream | TeX Live source tree, GitHub mirror <https://github.com/TeX-Live/texlive-source> |
| Tag | `tags/texlive-2026.1` ("texlive-2026.1 tag based on r78399") |
| Commit | `6a300188053b8f2ded89dbd52293732a706b9c0e` |
| Fetched | 2026-09-29 |

Paths below this directory are the upstream paths, so a test script's
`$srcdir` (`texk/web2c`) and `$srcdir/../kpathsea` resolve as in TeX Live's
own `make check`. Checksums: `shasum -a 256 -c SHA256SUMS` here.

## The tests

`pdftex_tests` of `texk/web2c/pdftexdir/am/pdftex.am`, in its order:

| test | what it checks | files it reads |
|---|---|---|
| `pdftexdir/wprob.test` | a `file:line:error` message from inside `\scantokens` names the enclosing file and line | `tests/wprob.tex` |
| `pdftexdir/pdftex.test` | `--version` and `--help` work | |
| `pdftexdir/pdfimage.test` | a format dumps, and JPEG, PDF and PNG images are included | `pdftexdir/tests/pdfimage.tex`, `tests/basic.tex`, `tests/1-4.jpg`, `tests/B.pdf`, `tests/lily-ledger-broken.png` |
| `pdftexdir/expanded.test` | `\expanded` (the log between START and END equals `expanded.txt`) | `pdftexdir/tests/expanded.tex`, `pdftexdir/tests/expanded.txt` |
| `pdftexdir/tests/cnfline.test` | `--cnf-line=max_print_line=500` takes effect | `pdftexdir/tests/cnfline.tex` |
| `pdftexdir/tests/partoken.test` | `\partokenname` and `\partokencontext` succeed and fail where they should | `tests/partoken-ok.tex`, `tests/partoken-xfail.tex`, `tests/cmr10.tfm` |
| `pdftexdir/wcfname.test` | Unicode file names, `\input\|cmd`, `--recorder`, `--jobname` | `tests/fn-generate.perl` (run with perl); needs `kpsewhich` |

All of them set `TEXMFCNF=$srcdir/../kpathsea`, so `texk/kpathsea/texmf.cnf`
is here too. Not included: `pdftosrc.test` and `ttf2afm.test`, which test
other programs (`pdftosrc`, `ttf2afm`) and are not in `pdftex_tests`.

## Licences

The test scripts and TeX files state their terms in their headers ("You may
freely use, modify and/or distribute this file", public domain);
`texk/kpathsea/texmf.cnf` is public domain; `tests/cmr10.tfm` is Knuth's
Computer Modern metric; the three images are TeX Live's own test data. All
are kept byte-for-byte unmodified.
