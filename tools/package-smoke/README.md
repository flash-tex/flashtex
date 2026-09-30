# package-smoke

One small `.tex` per common LaTeX package (article class, one `\usepackage`, 3 to 8 lines using the package's main commands), so the candidate engine can be compared with pdfTeX on real package usage (DESIGN §8 T3 breadth). MIT; inputs only, expected output always comes from pdfTeX 1.40.29 at run time.

- `*.tex`: 57 documents. Every one compiles cleanly with `pdflatex -interaction=nonstopmode -halt-on-error` in the default (restricted) shell mode, at most 3 passes, exit 0, no `! ` line, no undefined reference or citation. Written by Muse lanes (public technical content), re-run independently before landing.
- `smoke-a-manifest.json`, `smoke-b-manifest.json`: per package the file, page count and passes, plus the packages that were skipped and why (`framed`, `preprint`, `overpic`, `xkeyval`).
- `run.py`: `FLASHTEX_POOL=... FLASHTEX_FORMATS=... python3 tools/package-smoke/run.py --candidate BIN [PKG ...]` runs each document through the lockstep `capture()` (format pdflatex, separate directory per engine) on the candidate and on pdfTeX and reports `equal` or `DIFFERENT`. It exits 1 when any document differs.

## Result (candidate built from main `11d364f75`, pdfTeX 1.40.29, TeX Live 2026)

57 documents, 56 equal, 1 different: `yfonts` (candidate exit 1, pdfTeX exit 0). Cause: `yfonts` loads `yinit`, a bitmap-only font with no `pdftex.map` entry; pdfTeX embeds it as Type 3, the candidate aborts (issue #1218, PK/Type 3 fonts not implemented yet).

## Adding a package

Same rules: installed (`kpsewhich <pkg>.sty`), under 40 lines, no external files, no images, no bibliography or index, no `\write18`, compiles cleanly on pdfTeX after at most two honest fixes to your own file. Never edit a file to hide an error.
