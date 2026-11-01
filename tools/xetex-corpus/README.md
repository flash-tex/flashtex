# Unicode-mode corpus: typical xelatex documents with no TeX Live

Lane UNICODE-INDEPENDENT (owner priority: Unicode mode must work with
FlashTeX's own bundle and the system's fonts alone; DESIGN.md D12,
`docs/design/xetex/PLAN.md` §3.4). 39 documents of the kinds xelatex users
write: 37 here (`docs/<id>/`) and two of TeX Live's own examples, copied
from the oracle's tree at run time (`TEXLIVE_DOCS` in `run.py`):

| group | documents |
|---|---|
| fontspec | `f01` Georgia, Helvetica Neue, Menlo (system); `f02` TeX Gyre **by name**; `f05` the same by file; `f03` Libertinus; `f04` EB Garamond's OpenType features; `x01` TeX Live's `fontspec-example.tex` |
| unicode-math | `m01` Latin Modern Math; `m02` STIX Two; `m03` Pagella Math with `range=` from STIX Two and Termes Math; `m04` Libertinus Math and siunitx; `m05` XITS; `m06` Fira Math |
| polyglossia | `p01` Greek; `p02` Russian (CM Unicode); `p03` Arabic (Amiri, RTL); `p04` Hebrew (David CLM, RTL); `p05` Hindi (system Kohinoor Devanagari); `p06` French, German, Spanish with csquotes |
| CJK | `c01` xeCJK (Fandol); `c02` xeCJK with system Songti SC and Hiragino Sans GB; `c03` ctexart; `c04` Japanese (Hiragino); `c05` ctexbeamer |
| beamer, graphics | `b01` metropolis; `b02` TikZ, pgfplots, hyperref, graphicx; `b03` columns, blocks, overlays, unicode-math; `g01` article with pgfplots and images |
| bibliographies | `r01` biblatex + biber; `r02` biblatex `backend=bibtex`; `r03` natbib + bibtex; `r04` biblatex-ieee + biber; `x02` TeX Live's biblatex `01-introduction.tex` (biber) |
| other | `t01` report: geometry, fancyhdr, siunitx, booktabs, csquotes, microtype, enumitem, titlesec, caption, cleveref; `t02` thesis with system Times New Roman/Arial, subcaption, listings; `t03` imakeidx (makeindex); `t04` minted; `t05` xltxtra/xunicode; `u01` characters Georgia lacks; `u02` hyperref with Unicode bookmarks |

## Running

```sh
python3 tools/xetex-corpus/run.py --v3 target/release/flashtex-v3 \
    --host-unicode crates/flashtex-xetex/target/release/flashtex-host-unicode \
    --out /tmp/xc --with-texlive [--only 'p0*'] [--jobs 3] [--json OUT] [--visual]
```

Per document:

* **ref**: TeX Live's `latexmk -xelatex` (the oracle; it runs biber, bibtex
  and makeindex as it decides). `TEXMFCONFIG` and `TEXMFSYSCONFIG` point at
  an empty directory, so the paper is TeX Live's own default (A4), which the
  bundle carries; MacTeX's installer sets `letter` there (`tlmgr paper`).
* **notex**: `flashtex-v3 build --mode unicode`, which starts
  `flashtex-host-unicode` and talks display-list-v3 to it, with TeX Live
  hidden as `scripts/notex-host-check.sh` hides it (empty environment, own
  HOME and TMPDIR, system PATH, `sandbox-exec` denying every TeX Live path),
  the bundle from `flashtex-bundle.lock` with the user's consent, and the
  user's `~/Library/Fonts` (`FLASHTEX_FONT_DIRS`; the app sees it with the
  real HOME). Then `flashtex-v3 check --json` for the diagnostics.
* **tl** (`--with-texlive`): the same build with TeX Live visible. A
  document that works here and not in notex has a bundle gap; one that
  differs here too has an engine or host difference.

PDFs are compared by `tools/xetex-pdfparity/compare.py` (structural; the
#1708 tools), the oracle only as the reference (DESIGN.md).

## Results (mac-m1max-a, 2026-10-11, bundle `texbundle-tl2026-3`)

VERIFIED by the run above with this branch's host. "same": 0 structural
differences. ToUnicode differences are the known class the #1708 baseline
allows; "Menlo (AAT)": macOS xelatex shapes Menlo with AAT (Core Text), which
is out of scope (PLAN.md §3, no Core Text), so `Scale=MatchLowercase` and
sizes differ.

| doc | works with no TeX Live | notex vs xelatex | tl vs xelatex | gap |
|---|---|---|---|---|
| b01 | yes | same | same | |
| b02 | yes | 89: 84 beamer navigation links missing, rule colour | same 89 | writer: `dvipdfmx:catch_phantom` not implemented |
| b03 | yes | 87 (69 as tl + 18 sub-0.03 bp shading placements) | 69: navigation links | writer: `catch_phantom`; the 18: bundle's beamer/pgf against MacTeX's newer ones (belief) |
| c01 | yes | 64 ToUnicode | same | |
| c02 | yes | 8 ToUnicode | same | |
| c03 | **no**: `zhnumber.sty` not found | - | same | bundle: `zhnumber` |
| c04 | yes | 66: ToUnicode, 10 glyphs from another Hiragino Sans face | same | name lookup without Core Text (PLAN.md §5 risk) |
| c05 | **no**: `zhnumber.sty` | - | 45: navigation links | bundle: `zhnumber`; writer: `catch_phantom` |
| f01 | yes | 188 glyph positions | same | Menlo (AAT) |
| f02 | as xelatex: nothing typeset | same | same | TeX Live's fonts by name: macOS xelatex does not find them either (Linux and Windows xelatex do, through fontconfig) |
| f03, f04, f05 | yes | f03: 6 ToUnicode; f04, f05 same | same | |
| g01 | yes | same | same | |
| m01-m04 | yes | same | same | |
| m05 | **no**: XITS fonts not found | 106 | same | bundle: `xits` |
| m06 | **no**: Fira Math not found | 99 | same | bundle: `firamath` |
| p01 | yes | same | same | |
| p02 | **no**: CMU fonts not found | 256 | same | bundle: `cm-unicode` |
| p03 | **no**: `bidi.sty` | - | 302: Arabic glyphs differ | bundle: `bidi`, `amiri`; engine: Arabic shaping |
| p04 | **no**: `bidi.sty` | - | 132: Hebrew glyphs and positions differ | bundle: `bidi`, `culmus`; engine: RTL |
| p05 | yes | 4 ToUnicode | same | |
| p06 | yes | same | same | |
| r01, r04, x02 | PDF without the bibliography: "biber not run: biber is not installed" | 950, 534, 2225 | same/3/1 | biber (below) |
| r02 | yes (bibtex built in) | same | same | fixed here: in-process bibtex, `JOB-blx.bib` in the output directory |
| r03 | yes (bibtex built in) | same | same | fixed here |
| t01 | yes | 1 ToUnicode | same | |
| t02 | yes | 52: Menlo (AAT), ToUnicode | same | |
| t03 | yes (makeindex built in) | same | same | fixed here |
| t04 | no highlighting: "minted v3+ executable is not installed" | 53 | 43: Menlo (AAT) | minted runs TeX Live's `latexminted` (Python, Pygments) through `\write18`; not independent |
| t05 | **no**: `xunicode.sty` | - | same | bundle: `xunicode`, `xltxtra`, `realscripts` |
| u01 | yes; the log's 14 `Missing character` lines equal xelatex's, no crash | same | same | |
| u02 | yes | same | same | |
| x01 | yes | same | same | (Inconsolata by name: not found by either) |

Fixed on this branch (each VERIFIED by the corpus and `tests/host.rs`):

1. **bibtex and makeindex with no TeX Live.** The Unicode host ran them only
   as TeX Live's programs, so with no TeX Live a bibliography or index was
   skipped. They are now the engine's in-process ports (as the pdfTeX host
   has them), run in a child of the host (`--run-tool flashtex:bibtex`):
   `.bbl/.blg/.ind/.ilg` byte-identical to TeX Live's on
   `fixtures/notex-tools`; `\write18` makeindex (imakeidx) too.
2. **bibtex's databases** are looked for in the project, the output
   directory (biblatex's `JOB-blx.bib` with `backend=bibtex`) and the TeX
   tree, as the pdfTeX host does; the project only before.
3. **Fonts at paths of 256 bytes or more** were drawn as nothing, without a
   diagnostic: XDV keeps the path's length in one byte, as XeTeX does. The
   writer now takes the engine's own path for such a font. (A bundle cache
   under a long home directory hits this.)

Not fixed here (issues): beamer's navigation links (`dvipdfmx:catch_phantom`),
Arabic and Hebrew shaping, the bundle's fonts by name, biber.
