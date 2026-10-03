# *An Infinite Descent into Pure Mathematics* on the new engine (2026-10-03)

Lane **INFDESC-ENGINE** (mac-claude-a, mac-m1max-a). Owner priority 2026-10-03:
FlashTeX must compile the whole of Clive Newstead's textbook *An Infinite Descent
into Pure Mathematics* (<https://infinitedescent.org>). DESIGN.md §1.1 (P-T1,
P-T2), §4.5 (restricted `\write18`), §8 (T3 corpus tiers).

**Engine under test:** `flashtex-initex` from main **`8aee5e3be`**, built by
`scripts/engine-parity.sh build` (release, `pdflatex.fmt` made by the engine
from TeX Live's `pdflatex.ini`). **Oracle:** TeX Live 2026 pdfTeX 1.40.29
(`/Library/TeX/texbin/pdftex`), MacTeX on mac-m1max-a, oracle only.

**Source:** Codeberg `cnewstead/infdesc` at commit
`48825c5e50bf311b818d666cece6e731d8a0191d` (the owner's local checkout, run
from a copy; the commit archive's tree was checked identical to it with
`diff -rq`). `infdesc.tex` plus 95 `.tex` files under `book/`. Class `book`
10pt with mathptmx, T1, babel british, tikz, tikz-cd, mdframed (TikZ),
ntheorem (thmmarks, framed), imakeidx (four indices), cleveref, hyperref,
bookmark, listings, pdfpages, textpos, textgreek, titlesec, fancyhdr, bbm,
bussproofs, pifont, lastpage, datetime, footmisc, enumitem, ulem, environ,
clipboard, type1cm and more.

## Headline (VERIFIED on mac-m1max-a)

| check | result |
|---|---|
| Completes | yes, every pass exits 0, no `!` error |
| Pages | **592**, equal to the oracle and to the shipped `infdesc.pdf` |
| Every pass's log (P-T1 normalisation, untraced) | **equal** on all 4 passes |
| Every pass's PDF | **byte-identical** to the oracle's on all 4 passes |
| P-T2 (`tiers.compare_pt2`, converged PDFs) | **pass**: 592/592 page content streams, 41/41 font programs |
| Aux files (`.aux`, `.toc`, 4× `.idx`/`.ind`/`.ilg`) | identical after every pass |
| Index generation | the engine runs `makeindex` 4× per pass itself, via restricted `\write18` (`runsystem(makeindex infdesc.idx)...executed safely (allowed).`), as pdflatex does |
| P-T1 (traced pass, `\tracingall` + box dumps) | **pass**: 592/592 box dumps equal, the 20.3 GB strict log equal ([P-T1](#p-t1-traced-pass)) |
| Divergences found | **none** |
| Engine fixes needed | **none** |

## Run sequence

The document needs no external step. imakeidx (`makeindex` option) runs
`makeindex` from inside each pass through restricted `\write18`, which is
TeX Live's default (`shell_escape=p`, `makeindex` is in
`shell_escape_commands`) and the engine's default (DESIGN.md §4.5). So the
sequence is `pdflatex` until the log asks for no rerun:

| pass | oracle (pdflatex) | engine | pages | rerun? |
|---|---|---|---|---|
| 1 | exit 0 | exit 0 | 588 | yes (1,808 warnings: undefined references on the first pass) |
| 2 | exit 0 | exit 0 | 592 | yes ("Label(s) may have changed") |
| 3 | exit 0 | exit 0 | 592 | no: converged |
| 4 | exit 0 | exit 0 | 592 | (check) PDF byte-identical to pass 3 |

Both engines ran as the parity harness runs them (`tools/parity/capture.py`):
argv[0] `pdftex` through a link, `-fmt=pdflatex -interaction=nonstopmode`, the
first line `\pdfsetrandomseed 1\relax\input{infdesc.tex}`,
`SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1`, `max_print_line=10000`,
`error_line=254`, `half_error_line=238`, default (restricted) `\write18`, each
in its own copy of the tree.

Logs were compared after `capture.normalise_log` (banner and work directory)
and `capture.split_accounting` (the DESIGN §1.1 accounting ruling). The only
accounting difference, which does not gate, is in the end-of-run capacity
totals:

```
- 57362 strings out of 467525          (oracle)
+ 57362 strings out of 467558          (engine)
- 1173333 string characters out of 5418982
+ 1173333 string characters out of 5420156
```

The strings used are equal. The totals are `max_strings` minus the format's
own strings, and the two `pdflatex.fmt` files were built separately (TeX
Live's by fmtutil on 2026-03-22; the engine's by `engine-parity.sh build`).

### The oracle's final log

The converged log (pass 3 and pass 4) has 0 errors, 4 warnings, 16 overfull
and 19 underfull boxes; the engine's log is identical:
- 2× `LaTeX Font Warning: Font shape 'T1/cmtt/m/n' in size <126.47249> not available`;
- `Package mathptmx Warning: There are no bold math fonts`;
- `LaTeX Font Warning: Size substitutions with differences`.

### The shipped `infdesc.pdf`

The PDF in the checkout has **592 pages**, the same as our oracle. It was made
on 2026-09-02 by pdfTeX 1.40.26 (`TeX Live 2025/dev/Debian`), so its fonts,
package versions, `/CreationDate` and `PTEX.Fullbanner` differ from a TeX Live
2026 run. The parity reference is our own pdflatex run (DESIGN: the oracle is
the local pdflatex), not the shipped file.

## P-T1 (traced pass)

One traced pass per engine on its converged tree (after pass 4), through
`tools/parity/capture.py`'s own `capture()` (the harness's `\tracingall`,
`\tracingonline=1`, `\showboxdepth=\showboxbreadth=2147483647`), with the
log streamed through a named pipe into its `pt1stream` fingerprint, so no
log reached the disk. Then `tiers.compare_pt1_streamed`:

| | oracle (pdfTeX 1.40.29) | engine (`8aee5e3be`) |
|---|---|---|
| traced log | 20,318,349,077 bytes, complete | 20,318,349,076 bytes, complete |
| wall time (loaded host, both at once) | 2,317 s | 2,057 s |
| shipouts | 592 | 592 |

**P-T1: pass.** All 592 shipout box dumps are equal (`boxes_equal`) and the
strict log is equal (`log_equal`). The one-byte difference in size is in the
accounting lines, which do not gate: the capacity totals above, in the
end-of-run block (606 accounting lines each).

## Parity corpus: the `books` tier

**Licence** (`LICENCES.md` in the source): dual. The source code (all `.tex`
files) is under **LPPL 1.3c** (status "maintained", maintainer Clive
Newstead); the compiled book, its text and figures are under **CC BY-SA 4.0**.
Some code fragments come from TeX - LaTeX Stack Exchange with their own
licences, each linked in place. Both licences would allow redistribution
with attribution. Even so, the project rule is that third-party sources are
never committed (`tools/parity/README.md`), so this PR commits only a
manifest that pins the source:

- `tools/parity/corpus/books.json`: tier `books`, one entry, the Codeberg
  commit archive of `48825c5`, pinned by SHA-256
  (`40570a5c…b929e`, 3,070,297 bytes; two fetches 3 s apart returned the same
  bytes; the harness run below fetched it again). `on_demand`, so a bare `corpus.py fetch` still fetches only the
  T3 tiers. The nightly workflow names its tiers, so it does not run `books`
  until the Commander adds it.
- `tools/parity/corpus.py`: an archive tier (`ARCHIVE_TIERS`) is fetched and
  verified as an e-print is (the cache's `archives/<id>`). An entry's `root`
  names the archive's top directory, which becomes the tree, so
  `\input{book/includes/_includes.tex}` resolves as in the checkout. A changed
  hash is reported, never accepted. A forge may regenerate an archive with
  other bytes; then the fetch fails loudly and the entry needs a new pin.
- Tests: `test_archive_tier_tree_is_its_root_directory` and
  `test_books_manifest_pins_a_commit_archive` (`tools/parity/test_parity.py`;
  the whole file passes, 132 tests).

Run it with:

```
python3 tools/parity/parity.py --tier books --engine <work>/eng/flashtex-initex \
  --engine-env FLASHTEX_FORMATS=<work>/fmt --engine-env FLASHTEX_POOL=<work>/eng/pdftex.pool \
  --pt on --raster none --require-pt
```

HARNESS-RUN-PENDING

## Divergences found and fixed

None. The engine at `8aee5e3be` matches pdfTeX 1.40.29 on this book with no
change, so this lane made no engine change and adds no lockstep case.

## What remains

- **Timing** was not measured on a quiet host. Both engines ran at the same
  time on a loaded machine (other agents' builds); the wall times per pass
  (engine 34/84/45/60 s, oracle 35/89/40/61 s) are equal within noise, so they
  are no benchmark.
- **The app path:** this lane measured the engine through its pdfTeX command
  line (`flashtex-initex`), as the parity harness does. The app's host
  (`flashtex-host`, incremental, display list) was not measured on this book.
- **The `books` tier in nightly** is the Commander's call (`nightly.yml`).

## Reproduce

```
CARGO_BUILD_JOBS=4 scripts/engine-parity.sh --work <work> build
# a copy of the source per engine; then per pass, in its directory:
pdftex -fmt=pdflatex -interaction=nonstopmode -jobname=infdesc '\pdfsetrandomseed 1\relax\input{infdesc.tex}'
# (argv[0] `pdftex` from a link to flashtex-initex, with FLASHTEX_FORMATS=<work>/fmt
#  FLASHTEX_POOL=<work>/eng/pdftex.pool; or to TeX Live's pdftex for the oracle)
```

or the `books` tier of `tools/parity/parity.py` above.
