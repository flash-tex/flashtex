# package-smoke

One small `.tex` per common LaTeX package (article class, one or two `\usepackage` lines, a handful of lines using the package's main commands), so the candidate engine can be compared with pdfTeX on real package usage (DESIGN §8 T3 breadth). MIT; inputs only, expected output always comes from pdfTeX 1.40.29 at run time.

- `*.tex`: 59 documents. Every one compiled cleanly with `pdflatex -interaction=nonstopmode -halt-on-error` in the default (restricted) shell mode when it was added: exit 0, no `! ` line, no undefined reference or citation. Written by Muse lanes (public technical content), re-run independently before landing. The `xkeyval` document is called `xkeyval-smoke.tex` because a job named `xkeyval.tex` shadows the package's own file when compiled directly.
- `smoke-a-manifest.json`, `smoke-b-manifest.json`: same schema (`packages` with file, package, pages and passes measured on pdfTeX; `skipped` with the reason). Skipped: `overpic` (needs an image argument), `preprint` (not installed).
- `run.py`: compares the candidate with pdfTeX (see below). `test_run.py`: its unit tests with fake engines (`python3 -m unittest discover -s tools/package-smoke`).

## What `run.py` checks

```sh
FLASHTEX_POOL=... FLASHTEX_FORMATS=... python3 tools/package-smoke/run.py --candidate BIN [DOC ...]
```

Each document is compiled twice, in the same fresh directory, by the candidate and by the reference (format pdflatex, lockstep `capture()`), and is `equal` only if on both passes: the exit codes agree; the compared transcripts agree, including the per-shipout box dumps (the runner sets `\tracingoutput` and the `\showbox` limits to the maximum in a small wrapper); both runs wrote a PDF that passes the lockstep integrity check (a run that writes none FAILS); and the PDFs are equal after `qpdf --qdf --object-streams=disable --deterministic-id` (P-T2; that `/ID` depends on content, not path, so nothing is stripped, and a missing or failing qpdf is a harness error). The reference must be pdfTeX 1.40.29. A missing, refusing or hanging candidate counts as DIFFERENT for that document. Exit status: 0 all equal, 1 some differ, 2 harness or usage error (unknown document name, empty selection, wrong reference version, qpdf missing or failing).

This is a comparison of transcripts, box dumps and PDFs, not the `tools/parity` P-T1/P-T2 numbers of the packages tier.

## Result (candidate built from main `11d364f75`, pdfTeX 1.40.29, TeX Live 2026)

59 documents, 58 equal, 1 different: `yfonts` (candidate exit 1, pdfTeX exit 0). `yfonts` loads `yinit`, a bitmap-only font with no `pdftex.map` entry; pdfTeX embeds it as Type 3, the candidate aborts (issue #1218, PK/Type 3 fonts not implemented yet).

## Adding a package

Installed (`kpsewhich <pkg>.sty`), under 40 lines, no external files, no images, no bibliography or index, no `\write18`, compiles cleanly on pdfTeX after at most two honest fixes to your own file. Never edit a file to hide an error.
